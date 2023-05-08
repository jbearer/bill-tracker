import React from 'react'
import { useSearchParams } from 'react-router-dom'
import { gql, useQuery } from '@apollo/client'
import { type DocumentNode } from 'graphql'

import { BILL_FIELDS } from 'components/bill'
import { ISSUE_FIELDS } from 'components/issue'
import { LEGISLATOR_FIELDS } from 'components/legislator'
import { SideMenu, SideMenuSection, SideMenuLink, SideMenuHeader } from 'components/side-menu'
import { renderGqlResponse } from 'helpers/gql'
import MainLayout from 'layouts/main'

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

  const menu =
    <SideMenu>
      <SideMenuSection>
        <SideMenuHeader>I&apos;m looking for...</SideMenuHeader>
        <SideMenuLink to={`/search/bills?query=${query}`}>Bills</SideMenuLink>
        <SideMenuLink to={`/search/issues?query=${query}`}>Issues</SideMenuLink>
        <SideMenuLink to={`/search/people?query=${query}`}>People</SideMenuLink>
      </SideMenuSection>
    </SideMenu>

  const res = useQuery(gqlQuery(type, query), { variables: {} })
  const content = renderGqlResponse(res)

  return (
    <MainLayout menu={menu}>
      {content}
    </MainLayout>
  )
}

function gqlQuery (type: SearchType, query: string): DocumentNode {
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
          bills {
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
          legislators {
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
          issues {
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
