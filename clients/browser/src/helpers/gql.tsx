import React from 'react'
import { type QueryResult, type OperationVariables } from '@apollo/client'

import { Entities } from 'components/entity'

export function renderGqlResponse<V extends OperationVariables> ({ loading, error, data }:
QueryResult<any, V>): React.ReactNode {
  if (loading) return <p>Loading...</p>
  if (error != null) return <p>Error : {error.message}</p>
  return <Entities data={data} />
}
