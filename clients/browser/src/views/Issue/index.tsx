import React from 'react'
import { useParams } from 'react-router-dom'

export default function Issue (): JSX.Element {
  const { id } = useParams()

  return (
    <table>
      <tr>
        <td>ID</td>
      </tr>
      <tr>
        <td>{id}</td>
      </tr>
    </table>
  )
}
