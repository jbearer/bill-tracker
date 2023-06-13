import React from 'react'

import Filters from './filters'

interface BillFiltersProps {
  onFilterChange: (filter: any) => void
}

export default function BillFilters (props: BillFiltersProps): JSX.Element {
  return <Filters
    onFilterChange={props.onFilterChange}
    filters={[
      {
        path: ['state', 'name'],
        resource: 'states',
        name: 'State'
      },
      {
        path: [{ name: 'issues', plural: true }, 'name'],
        resource: 'issues',
        name: 'Issues'
      }
    ]}
  />
}
