import React, { useState, useEffect } from 'react'
import { gql, useQuery } from '@apollo/client'

import { MultiSelect, fuzzyFilter } from 'components/multi-select'
import { SideMenuHeader, SideMenuSection, SideMenuItem } from 'components/side-menu'

export interface Filters {
  states: string[]
}

interface PeopleFiltersProps {
  onFilterChange: (filter: string) => void
}

export default function PeopleFilters (props: PeopleFiltersProps): JSX.Element {
  const states = useQuery(ALL_STATES_QUERY)

  if (states.loading) {
    return <p>Loading...</p>
  }
  if (states.error != null) {
    return <p>Error: {states.error.message}</p>
  }

  const stateNames = (states.data.states?.edges ?? []).map((edge: any) => edge.node?.name)

  return <PeopleFiltersWithData states={stateNames} {...props} />
}

interface PeopleFiltersWithDataProps extends PeopleFiltersProps {
  states: string[]
}

function PeopleFiltersWithData (props: PeopleFiltersWithDataProps): JSX.Element {
  const [filter, setFilter] = useState({
    states: []
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
  </>
}

function gqlFilter (filters: Filters): string {
  let statePred = ''
  if (filters.states.length !== 0) {
    const states = filters.states.map((name) => `{ lit: "${name}" }`).join(',')
    statePred = `district: {
      has: {
        state: {
          has: {
            name: {
              in: [${states}]
            }
          }
        }
      }
    }`
  }

  return `{
    has: {
      ${statePred}
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
