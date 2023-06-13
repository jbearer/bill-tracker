import React from 'react'

import Filters from './filters'

interface PeopleFiltersProps {
  onFilterChange: (filter: any) => void
}

export default function PeopleFilters (props: PeopleFiltersProps): JSX.Element {
  return <Filters
    onFilterChange={props.onFilterChange}
    filters={[
      {
        path: ['district', 'state', 'name'],
        resource: 'states',
        name: 'State'
      }
    ]}
  />
}
