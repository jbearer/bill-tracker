import React, { useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { gql, useQuery } from '@apollo/client'
import { type DocumentNode } from 'graphql'

import { BILL_FIELDS } from 'components/bill'
import { ISSUE_FIELDS } from 'components/issue'
import { LEGISLATOR_FIELDS } from 'components/legislator'
import { SideMenu, SideMenuSection, SideMenuNavLink, SideMenuHeader } from 'components/side-menu'
import { renderGqlResponse } from 'helpers/gql'
import MainLayout from 'layouts/main'
import BillFilters from './components/bill-filters'
import PeopleFilters from './components/people-filters'

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
  const content = renderGqlResponse(res)

  return (
    <MainLayout menu={menu}>
      {content}
    </MainLayout>
  )
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
          bills {
            edges {
              node {
                ...BillFields
              }
            }
          }
          legislators {
            edges {
              node {
                ...LegislatorFields
              }
            }
          }
          issues {
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
