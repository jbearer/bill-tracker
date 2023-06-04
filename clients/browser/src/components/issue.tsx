import React from 'react'
import { gql } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { Link } from 'react-router-dom'

import { Card, Title, Body } from 'components/card'

interface Props {
  data: any
}

export const ISSUE_FIELDS: DocumentNode = gql`
  fragment IssueFields on Issue {
    id
    name
  }
`

/// Parse a GraphQL `Issue` object and render it.
export default function Issue ({ data }: Props): JSX.Element {
  const id = data.id
  if (typeof id !== 'number') {
    console.log('Invalid type of id', id)
    return <Card>Invalid data</Card>
  }

  return <Card>
    <Title>
      <Link to={`/issues/${id}`}>
        {data.name} (issue)
      </Link>
    </Title>
    <Body>
    </Body>
  </Card>
}
