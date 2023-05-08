import React from 'react'
import { useParams } from 'react-router-dom'
import { useQuery, gql } from '@apollo/client'

import MainLayout from 'layouts/main'
import { ISSUE_FIELDS } from 'components/issue'
import { renderGqlResponse } from 'helpers/gql'

export default function Issue (): JSX.Element {
  const { id } = useParams()
  if (id == null) {
    throw new Error('missing required route parameter "id"')
  }

  const res = useQuery(gql`
    ${ISSUE_FIELDS}
    query GetIssue($id: Int) {
      issues(where: { has: { id: { is: { lit: $id } } } }) {
        edges {
          node {
            ...IssueFields
          }
        }
      }
    }
  `, {
    variables: { id: +id }
  })

  const content = renderGqlResponse(res)

  return (
    <MainLayout>{content}</MainLayout>
  )
}
