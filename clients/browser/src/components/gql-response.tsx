import React from 'react'
import { type QueryResult, type OperationVariables } from '@apollo/client'

import { Entities } from 'components/entity'

interface GqlResponseProps<V extends OperationVariables> {
  response: QueryResult<any, V>
}

export default function GqlResponse<V extends OperationVariables> (
  { response: { loading, error, data } }: GqlResponseProps<V>): JSX.Element {
  if (loading) return <p>Loading...</p>
  if (error != null) return <p>Error : {error.message}</p>
  return <Entities data={data} />
}
