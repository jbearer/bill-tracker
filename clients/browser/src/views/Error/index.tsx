import React from 'react'
import { useRouteError } from 'react-router-dom'

import MainLayout from 'layouts/main'

export default function Error (): JSX.Element {
  const error: any = useRouteError()
  console.error(error)

  return (
    <MainLayout>
      <div id="error-page">
        <h1>Oops!</h1>
        <p>Sorry, an unexpected error has occurred.</p>
        <p>
          <i>{error.statusText ?? error.message}</i>
        </p>
      </div>
    </MainLayout>
  )
}
