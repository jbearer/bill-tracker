// A generalized component for adding resource-specific filters in a side menu.
//
// The `Filters` component takes as a property a description of the filters that can be applied to
// a resource, renders UI components to allow the user to input values for any and all of these
// filters, and handles the construction of a GraphQL predicate corresponding to the user's input in
// these components. The GraphQL predicate is propagated to the parent component via the
// `onFilterChange` callback.
//
// A filter consists of
// * The path from the resource being filtered to a string-valued scalar field. This path may
//   encompass multiple levels of nested resources, as in `legislators.district.state.name`, or it
//   may even be a scalar field on the resource itself, as in `state.name`. Each segment of the path
//   may optionally be denoted as plural; this just affects the generated GraphQL predicate --
//   plural fields will be filtered via an `any` predicate, which takes as input the singular
//   predicate implied by the user's input.
// * The name of the resource owning the field. In other words, the type of the second-to-last
//   element in the past (since the last element denotes a scalar field). This is used to query all
//   values of that resource in order to find valid options for the filter.
// * An optional name which is used to label the UI component corresponding to the filter.
//
// For each filter, the component renders a multi-select where the user can select the values of the
// corresponding field that they want their search results to have. Each time the selected set of
// any of the multi-selects changes, the GraphQL predicate is recomputed and the `onFilterChange`
// event fires. The GraphQL predicate filters elements of the base type based on whether they
// satisfy an `in` predicate with the list of selected options. That is, the predicate will match
// any resource where the nested field denoted by the filter's path has a value which matches any of
// the selected options for that filter. If multiple filters are present, the result set will
// consist of objects matching _all_ of the filters.

import React, { useState } from 'react'
import { gql, useQuery, type DocumentNode } from '@apollo/client'

import { MultiSelect, fuzzyFilter } from 'components/multi-select'
import { SideMenuHeader, SideMenuSection, SideMenuItem } from 'components/side-menu'

// A segment of a filter path.
export interface PathSegment {
  name: string
  plural?: boolean
}

// Shorthand for a `PathSegment`.
//
// Singular path segments can be denoted simply by giving there name as a string. Plural segments
// must use the long-form `PathSegment` type and explicitly set `plural: true`.
export type PathSegmentDescriptor = string | PathSegment

// A description of a filter.
export interface FilterDescriptor {
  path: PathSegmentDescriptor[]
  resource: string
  name?: string
}

// Internal representation of a filter.
class Filter {
  _path: PathSegment[]
  _resource: string
  _name?: string

  constructor (desc: FilterDescriptor) {
    this._path = desc.path.map((segment) =>
      typeof segment === 'string'
        ? { name: segment }
        : segment
    )
    this._resource = desc.resource
    this._name = desc.name
  }

  // A unique identifier for this filter.
  key (): string {
    return this._path[0].name
  }

  // A human-redable name for this filter.
  name (): string {
    return this._name ?? this.key()
  }

  // The resource whose items are valid options for this filter.
  optionsResource (): string {
    return this._resource
  }

  // The field of `optionsResource` containing the options for this filter.
  optionsField (): string {
    return this._path[this._path.length - 1].name
  }

  // Extract the valid options for this filter from a GraphQL response containing (at least) all
  // entries in this filter's `optionsResource` with (at least) `optionsField` selected.
  getOptions (allOptions: any): string[] {
    return allOptions[this.optionsResource()]
      .edges
      .map((edge: any) => edge.node[this.optionsField()])
  }

  // Build a GraphQL predicate representing this filter based on the selected options.
  build (selected: string[], i: number = 0): any {
    const segment = this._path[i]

    // Create an object with a predicate on this segment.
    const res: any = {}
    res[segment.name] = {}

    // If the segment is plural, make it an `any` predicate, and focus on the argument of that
    // (which is a singular predicate).
    let pred = res[segment.name]
    if (segment.plural ?? false) {
      pred.any = {}
      pred = pred.any
    }
    // Now `pred` is a singular predicate, whether it is the overall predicate for a singular path
    // segment or the inner predicate of a plural predicate.

    if (i + 1 >= this._path.length) {
      // If this is the last path segment, it denotes a scalar field. Create a predicate which
      // checks if the scalar value is in our selected list.
      pred.in = selected.map((lit) => ({ lit }))
    } else {
      // Otherwise, this is a resource field. We filter it by a nested predicate which is
      // constructed recursively from the remainder of the path.
      pred.has = this.build(selected, i + 1)
    }

    return res
  }
}

interface FiltersProps {
  filters: FilterDescriptor[]
  onFilterChange: (filter: any) => void
}

export default function Filters (props: FiltersProps): JSX.Element {
  // Use GraphQL to query valid options for each filter.
  const filters = props.filters.map((desc) => new Filter(desc))
  const options = useQuery(allOptionsQuery(filters))

  if (options.loading) {
    return <p>Loading...</p>
  }
  if (options.error != null) {
    return <p>Error: {options.error.message}</p>
  }

  // Once the query for options completes successfully, render the filter UI.
  return <FiltersWithOptions
    options={options.data}
    filters={filters}
    onFilterChange={props.onFilterChange}
  />
}

interface FiltersWithOptionsProps {
  filters: Filter[]
  onFilterChange: (filter: any) => void
  options: any
}

function FiltersWithOptions (props: FiltersWithOptionsProps): JSX.Element {
  // We will keep track of the set of selected options for each filter, so that when one of the
  // selected sets changes, we can remember the previous (and still current) values of the other
  // ones. Initially, we populate this with an empty list for each filter.
  const [selected, setSelected] = useState(() =>
    Object.fromEntries(props.filters.map((filter) => [filter.key(), []])))

  // Render a multi-select for each filter.
  return <>{
    props.filters.map((filter, i) =>
      <SideMenuSection key={i}>
        <SideMenuHeader>{filter.name()}</SideMenuHeader>
        <SideMenuItem>
          <MultiSelect
            filter={fuzzyFilter(filter.getOptions(props.options))}
            onChange={(selectedOptions) => {
              // When one of the filters changes, updated `selected` with the new selected set for
              // that filter, keeping the same selected set for the other filters.
              const newSelected = Object.create(selected)
              newSelected[filter.key()] = selectedOptions
              setSelected(newSelected)

              // Recompute the GraphQL predicate corresponding to our current selection set and
              // update the parent.
              props.onFilterChange(gqlFilter(props.filters, newSelected))
            }}
          />
        </SideMenuItem>
      </SideMenuSection>
    )
  }</>
}

function gqlFilter (filters: Filter[], selected: Record<string, string[]>): any {
  const res: any = { has: {} }
  for (const filter of filters) {
    const options = selected[filter.key()]
    if (options.length > 0) {
      Object.assign(res.has, filter.build(options))
    }
  }

  return res
}

function allOptionsQuery (filters: Filter[]): DocumentNode {
  const queries: Record<string, string> = {}

  for (const filter of filters) {
    queries[filter.optionsResource()] = `{
      edges {
        node {
          id
          ${filter.optionsField()}
        }
      }
    }`
  }
  const fragments = Object.entries(queries).map(([key, fragment]) => `${key} ${fragment}`).join(' ')
  return gql`query { ${fragments} }`
}
