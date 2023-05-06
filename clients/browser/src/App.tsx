import React from 'react'
import { Outlet, ScrollRestoration } from 'react-router-dom'

export default function App (): JSX.Element {
  return (
    <div>
      <Outlet />
      <ScrollRestoration />
    </div>
  )
}
