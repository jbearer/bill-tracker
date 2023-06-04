import React from 'react'
import { gql } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { Link } from 'react-router-dom'

import { Card, Title, Body } from 'components/card'

interface Props {
  data: any
}

export const LEGISLATOR_FIELDS: DocumentNode = gql`
  fragment LegislatorFields on Legislator {
    id
    district {
      name
      state { abbreviation }
    }
    party {
      abbreviation
    }
    firstName
    lastName
  }
`

/// Parse a GraphQL `Legislator` object and render it.
export default function Legislator ({ data }: Props): JSX.Element {
  const id = data.id
  if (typeof id !== 'number') {
    console.log('Invalid type of id', id)
    return <Card>Invalid data</Card>
  }

  return <Card>
    <Title>
      <Link to={`/legislators/${id}`}>
        {data.firstName} {data.lastName}
        ({data.party.abbreviation}&ndash;{data.district.state.abbreviation})
      </Link>
    </Title>
    <Body>
      District: {data.district.name}
    </Body>
  </Card>
}
