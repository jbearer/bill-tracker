import React from 'react'
import { gql } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { Link } from 'react-router-dom'

import { Card, Title, Body } from 'components/card'

interface Props {
  data: any
}

export const BILL_FIELDS: DocumentNode = gql`
  fragment BillFields on Bill {
    id
    state { abbreviation }
    name
    title
    summary
    issues {
      edges {
        node {
          name
        }
      }
    }
    sponsors {
      edges {
        node {
          id
          firstName
          lastName
        }
      }
    }
  }
`

/// Parse a GraphQL `Bill` object and render it.
export default function Bill ({ data }: Props): JSX.Element {
  const id = data.id
  if (typeof id !== 'number') {
    console.log('Invalid type of id', id)
    return <Card>Invalid data</Card>
  }

  return <Card>
    <Title>
      <Link to={`/bills/${id}`}>
        {data.state.abbreviation} {data.name} &mdash; {data.title}
      </Link>
    </Title>
    <Body>
      Summary: {data.summary}
    </Body>
  </Card>
}
