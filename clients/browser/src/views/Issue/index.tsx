import React from 'react'
import { useParams } from 'react-router-dom'

import MainLayout from 'layouts/main'

export default function Issue (): JSX.Element {
  const { id } = useParams()

  return (
    <MainLayout>
      <table>
        <tr>
          <td>ID</td>
        </tr>
        <tr>
          <td>{id}</td>
        </tr>
      </table>
    </MainLayout>
  )
}
