import React from 'react'
import { useParams } from 'react-router-dom'
import { useQuery, gql } from '@apollo/client'

import MainLayout from 'layouts/main'
import { BILL_FIELDS } from 'components/bill'
import { renderGqlResponse } from 'helpers/gql'

export default function Bill (): JSX.Element {
  const { id } = useParams()
  if (id == null) {
    throw new Error('missing required route parameter "id"')
  }

  const res = useQuery(gql`
    ${BILL_FIELDS}
    query GetBill($id: Int) {
      bills(where: { has: { id: { is: { lit: $id } } } }) {
        edges {
          node {
            ...BillFields
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
