import React, { useState, useEffect } from 'react'
import { gql, useQuery } from '@apollo/client'

import { MultiSelect, fuzzyFilter } from 'components/multi-select'
import { SideMenuHeader, SideMenuSection, SideMenuItem } from 'components/side-menu'

export interface Filters {
  states: string[]
  issues: string[]
}

interface BillFiltersProps {
  onFilterChange: (filter: string) => void
}

export default function BillFilters (props: BillFiltersProps): JSX.Element {
  const states = useQuery(ALL_STATES_QUERY)
  const issues = useQuery(ALL_ISSUES_QUERY)

  if (states.loading) {
    return <p>Loading...</p>
  }
  if (states.error != null) {
    return <p>Error: {states.error.message}</p>
  }

  if (issues.loading) {
    return <p>Loading...</p>
  }
  if (issues.error != null) {
    return <p>Error: {issues.error.message}</p>
  }

  const stateNames = (states.data.states?.edges ?? []).map((edge: any) => edge.node?.name)
  const issueNames = (issues.data.issues?.edges ?? []).map((edge: any) => edge.node?.name)

  return <BillFiltersWithData states={stateNames} issues={issueNames} {...props} />
}

interface BillFiltersWithDataProps extends BillFiltersProps {
  states: string[]
  issues: string[]
}

function BillFiltersWithData (props: BillFiltersWithDataProps): JSX.Element {
  const [filter, setFilter] = useState({
    states: [],
    issues: []
  })
  useEffect(() => { props.onFilterChange(gqlFilter(filter)) })

  return <>
    <SideMenuSection>
      <SideMenuHeader>States</SideMenuHeader>
      <SideMenuItem>
        <MultiSelect filter={fuzzyFilter(props.states)}
          onChange={(selected) => {
            const newFilter = Object.create(filter)
            newFilter.states = selected
            setFilter(newFilter)
            props.onFilterChange(gqlFilter(newFilter))
          }}
        />
      </SideMenuItem>
    </SideMenuSection>
    <SideMenuSection>
      <SideMenuHeader>Issues</SideMenuHeader>
      <SideMenuItem>
        <MultiSelect filter={fuzzyFilter(props.issues)}
          onChange={(selected) => {
            const newFilter = Object.create(filter)
            newFilter.issues = selected
            setFilter(newFilter)
            props.onFilterChange(gqlFilter(newFilter))
          }}
        />
      </SideMenuItem>
    </SideMenuSection>
  </>
}

function gqlFilter (filters: Filters): string {
  let statePred = ''
  if (filters.states.length !== 0) {
    const states = filters.states.map((name) => `{ lit: "${name}" }`).join(',')
    statePred = `state: {
      has: {
        name: {
          in: [${states}]
        }
      }
    }`
  }

  let issuesPred = ''
  if (filters.issues.length !== 0) {
    const issues = filters.issues.map((name) => `{ lit: "${name}" }`).join(',')
    issuesPred = `issues: {
      any: {
        has: {
          name: {
            in: [${issues}]
          }
        }
      }
    }`
  }

  return `{
    has: {
      ${statePred}
      ${issuesPred}
    }
  }`
}

const ALL_STATES_QUERY = gql`
query {
  states {
    edges {
      node {
        name
      }
    }
  }
}
`

const ALL_ISSUES_QUERY = gql`
query {
  issues {
    edges {
      node {
        name
      }
    }
  }
}
`
