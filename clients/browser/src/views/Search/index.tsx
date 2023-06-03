import React, { useState } from 'react'
import { useSearchParams, Link } from 'react-router-dom'
import { gql, useQuery, type QueryResult } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { createUseStyles } from 'react-jss'

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
  const [filter, setFilter] = useState('{ has: {} }')

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

  const res = useQuery(gqlQuery(type, query, filter), { variables: {} })
  const content = type === SearchType.All
    ? <Preview response={res} query={query} />
    : <GqlResponse response={res} />

  return (
    <MainLayout menu={menu}>
      {content}
    </MainLayout>
  )
}

interface ResultsProps {
  response: QueryResult
}

const usePreviewStyles = createUseStyles((theme: Theme) => ({
  section: {
    ...theme.surface({ border: true }),
    borderStyle: 'solid',
    borderWidth: '1px',
    borderRadius: '15px',
    margin: '15px'
  },
  header: {
    padding: '10px',
    margin: '0',
    ...theme.secondary()
  },
  seeMore: {
    ...theme.secondaryLight({ activateOnHover: true }),
    textDecoration: 'none',
    borderRadius: '5px',

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

  const bills = response.data.bills
  const people = response.data.legislators
  const issues = response.data.issues

  return <>
    <PreviewSection name="Bills" data={{ edges: bills }} url={`/search/bills?query=${query}`} />
    <PreviewSection name="People" data={{ edges: people }} url={`/search/people?query=${query}`}/>
    <PreviewSection name="Issues" data={{ edges: issues }} url={`/search/issues?query=${query}`} />
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

function gqlFilters (type: SearchType, setFilter: (filter: string) => void): JSX.Element {
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

function gqlQuery (type: SearchType, query: string, filter: string): DocumentNode {
  switch (type) {
    case SearchType.All:
      return gql`
        ${BILL_FIELDS}
        ${LEGISLATOR_FIELDS}
        ${ISSUE_FIELDS}
        query SearchAll {
          bills(first: ${PREVIEW_COUNT}) {
            edges {
              node {
                ...BillFields
              }
            }
          }
          legislators(first: ${PREVIEW_COUNT}) {
            edges {
              node {
                ...LegislatorFields
              }
            }
          }
          issues(first: ${PREVIEW_COUNT}) {
            edges {
              node {
                ...IssueFields
              }
            }
          }
        }
      `
    case SearchType.Bills:
      return gql`
        ${BILL_FIELDS}
        query SearchBills {
          bills(where: ${filter}) {
            edges {
              node {
                ...BillFields
              }
            }
          }
        }
      `
    case SearchType.People:
      return gql`
        ${LEGISLATOR_FIELDS}
        query SearchPeople {
          legislators(where: ${filter}) {
            edges {
              node {
                ...LegislatorFields
              }
            }
          }
        }
      `
    case SearchType.Issues:
      return gql`
        ${ISSUE_FIELDS}
        query SearchIssues {
          issues(where: ${filter}) {
            edges {
              node {
                ...IssueFields
              }
            }
          }
        }
      `
  }
}
