import React, { useState } from 'react'
import { useSearchParams, Link } from 'react-router-dom'
import { gql, useQuery, type QueryResult } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { createUseStyles } from 'react-jss'
import InfiniteScroll from 'react-infinite-scroll-component'

import { BILL_FIELDS } from 'components/bill'
import { ISSUE_FIELDS } from 'components/issue'
import { LEGISLATOR_FIELDS } from 'components/legislator'
import { SideMenu, SideMenuSection, SideMenuNavLink, SideMenuHeader } from 'components/side-menu'
import GqlResponse from 'components/gql-response'
import MainLayout from 'layouts/main'
import BillFilters from './components/bill-filters'
import PeopleFilters from './components/people-filters'
import { Entities } from 'components/entity'
import { type Theme } from 'themes/theme'

const PREVIEW_COUNT = 5
const PAGE_COUNT = 50

export enum SearchType {
  All,
  Bills,
  People,
  Issues,
}

interface SearchProps {
  type: SearchType
}

export default function Search ({ type }: SearchProps): JSX.Element {
  const params = useSearchParams()[0]
  const query = params.get('query') ?? ''
  const [filter, setFilter] = useState<any>(() => ({ has: {} }))

  return <SearchWithFilter
    type={type}
    query={query}
    filter={filter}
    setFilter={setFilter}
  />
}

interface SearchWithFilterProps extends SearchProps {
  query: string
  filter: any
  setFilter: (filter: any) => void
}

function SearchWithFilter ({ type, query, filter, setFilter }: SearchWithFilterProps): JSX.Element {
  const variables: Record<string, any> = { query, filter }
  const res = useQuery(gqlQuery(type), { variables })

  const menu =
    <SideMenu>
      <SideMenuSection>
        <SideMenuHeader>I&apos;m looking for...</SideMenuHeader>
        <SideMenuNavLink to={`/search/bills?query=${query}`}>Bills</SideMenuNavLink>
        <SideMenuNavLink to={`/search/issues?query=${query}`}>Issues</SideMenuNavLink>
        <SideMenuNavLink to={`/search/people?query=${query}`}>People</SideMenuNavLink>
      </SideMenuSection>
      {gqlFilters(type, setFilter)}
    </SideMenu>

  const resKey = gqlResponseKey(type)
  const content = resKey === undefined
    ? <Preview response={res} query={query} />
    : <Results response={res} entity={resKey} />

  return (
    <MainLayout menu={menu}>
      {content}
    </MainLayout>
  )
}

interface ResultsProps {
  response: QueryResult
}

function Results ({ response, entity }: ResultsProps & { entity: string }): JSX.Element {
  const data = response.data?.[entity]
  const pageInfo = data?.pageInfo ?? {
    hasNextPage: true,
    endCursor: undefined
  }
  const length = Array.from(data?.edges ?? []).length

  return <InfiniteScroll
    dataLength={length}
    next={async () => {
      await response.fetchMore({
        variables: {
          cursor: pageInfo.endCursor
        }
      })
    }}
    hasMore={pageInfo.hasNextPage}
    loader={<p>Loading...</p>}
    hasChildren={!response.loading}
  >
    <GqlResponse response={response} />
  </InfiniteScroll>
}

const usePreviewStyles = createUseStyles((theme: Theme) => ({
  section: {
    ...theme.surface({ border: { radius: '15px' } }),
    margin: '15px'
  },
  header: {
    padding: '10px',
    margin: '0',
    ...theme.secondary()
  },
  seeMore: {
    textDecoration: 'none',
    ...theme.secondaryLight({ activateOnHover: true, border: { radius: '5px', width: 0 } }),

    margin: '10px',
    padding: '10px',

    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center'
  }
}))

function Preview ({ response, query }: ResultsProps & { query: string }): JSX.Element {
  if (response.loading) return <p>Loading...</p>
  if (response.error != null) return <p>Error : {response.error.message}</p>

  const bills = {
    bills: {
      edges: Array.from(response.data.bills.edges).slice(0, PREVIEW_COUNT)
    }
  }
  const people = {
    people: {
      edges: Array.from(response.data.legislators.edges).slice(0, PREVIEW_COUNT)
    }
  }
  const issues = {
    issues: {
      edges: Array.from(response.data.issues.edges).slice(0, PREVIEW_COUNT)
    }
  }

  return <>
    <PreviewSection name="Bills" data={bills} url={`/search/bills?query=${query}`} />
    <PreviewSection name="People" data={people} url={`/search/people?query=${query}`}/>
    <PreviewSection name="Issues" data={issues} url={`/search/issues?query=${query}`} />
  </>
}

interface SectionProps {
  name: string
  data: any
  url: string
}

function PreviewSection ({ name, data, url }: SectionProps): JSX.Element {
  const classes = usePreviewStyles()

  return <div className={classes.section}>
    <h3 className={classes.header}>{name}</h3>
    <Entities data={data} />
    <Link className={classes.seeMore} to={url}>See more</Link>
  </div>
}

function gqlFilters (type: SearchType, setFilter: (filter: any) => void): JSX.Element {
  switch (type) {
    case SearchType.All: {
      return <React.Fragment />
    }
    case SearchType.Bills: {
      return <BillFilters onFilterChange={setFilter} />
    }
    case SearchType.People: {
      return <PeopleFilters onFilterChange={setFilter} />
    }
    case SearchType.Issues: {
      return <React.Fragment />
    }
  }
}

function gqlQuery (type: SearchType): DocumentNode {
  const entityQuery = (name: string, predicate: string, fields: DocumentNode, fieldsFragment: string): DocumentNode => gql`
    ${fields}
    query search${name}($cursor: String, $query: String, $filter: ${predicate}) {
      ${name}(
        where: {
          all: [
            $filter,
            { matches: $query },
          ]
        },
        first: ${PAGE_COUNT},
        after: $cursor,
      ) {
        edges {
          node {
            ...${fieldsFragment}
          }
        }
        pageInfo {
          endCursor
          hasNextPage
        }
      }
    }
  `

  switch (type) {
    case SearchType.All:
      return gql`
        ${BILL_FIELDS}
        ${LEGISLATOR_FIELDS}
        ${ISSUE_FIELDS}
        query SearchAll($query: String) {
          bills(first: ${PREVIEW_COUNT}, where: { matches: $query }) {
            edges {
              node {
                ...BillFields
              }
            }
          }
          legislators(first: ${PREVIEW_COUNT}, where: { matches: $query }) {
            edges {
              node {
                ...LegislatorFields
              }
            }
          }
          issues(first: ${PREVIEW_COUNT}, where: { matches: $query }) {
            edges {
              node {
                ...IssueFields
              }
            }
          }
        }
      `
    case SearchType.Bills:
      return entityQuery('bills', 'BillPredicate', BILL_FIELDS, 'BillFields')
    case SearchType.People:
      return entityQuery('legislators', 'LegislatorPredicate', LEGISLATOR_FIELDS, 'LegislatorFields')
    case SearchType.Issues:
      return entityQuery('issues', 'IssuePredicate', ISSUE_FIELDS, 'IssueFields')
  }
}

function gqlResponseKey (type: SearchType): string | undefined {
  switch (type) {
    case SearchType.All: return undefined
    case SearchType.Bills: return 'bills'
    case SearchType.People: return 'legislators'
    case SearchType.Issues: return 'issues'
  }
}
