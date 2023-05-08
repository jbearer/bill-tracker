import React from 'react'
import { useParams } from 'react-router-dom'
import { useQuery, gql } from '@apollo/client'

import MainLayout from 'layouts/main'
import { LEGISLATOR_FIELDS } from 'components/legislator'
import { renderGqlResponse } from 'helpers/gql'

export default function Legislator (): JSX.Element {
  const { id } = useParams()
  if (id == null) {
    throw new Error('missing required route parameter "id"')
  }

  const res = useQuery(gql`
    ${LEGISLATOR_FIELDS}
    query GetLegislator($id: Int) {
      legislators(where: { has: { id: { is: { lit: $id } } } }) {
        edges {
          node {
            ...LegislatorFields
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
