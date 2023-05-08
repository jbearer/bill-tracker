/// Renderers for GraphQL responses.
import React from 'react'

import Bill from 'components/bill'
import Issue from 'components/issue'
import Legislator from 'components/legislator'

interface Props {
  data: any
}

/// Parse a GraphQL `response` and display all the objects within it.
///
/// `data` should contain one more more top-level keys mapping to Relay-style paginated connection
/// objects. Each item in each of these paginated connections is extracted and rendered as an
/// `Entity`.
export function Entities ({ data }: Props): JSX.Element {
  const entities = Object.values(data).flatMap((connection: any) => {
    const edges = connection.edges
    if (edges == null) {
      return []
    }

    return edges.flatMap((edge: any) => {
      const node = edge.node
      if (node == null) {
        return []
      }

      return [<Entity key={node.id} data={node} />]
    })
  })

  return <React.Fragment>{entities}</React.Fragment>
}

/// Parse a GraphQL object and render it.
///
/// The object will be rendered as a `Bill`, `Legislator`, etc. based on its GraphQL type.
export function Entity ({ data }: Props): JSX.Element {
  switch (data.__typename) {
    case 'Bill': return <Bill data={data} />
    case 'Issue': return <Issue data={data} />
    case 'Legislator': return <Legislator data={data} />
    default: {
      console.log('invalid data', data)
      return <React.Fragment />
    }
  }
}
