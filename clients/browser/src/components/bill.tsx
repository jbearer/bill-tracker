import React from 'react'
import { gql } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { createUseStyles } from 'react-jss'
import { Link } from 'react-router-dom'

import { type Theme } from 'themes/theme'

interface Props {
  data: any
}

export const BILL_FIELDS: DocumentNode = gql`
  fragment BillFields on Bill {
    id
    state { abbreviation }
    name
    title
    summary
    issues {
      edges {
        node {
          name
        }
      }
    }
    sponsors {
      edges {
        node {
          id
          firstName
          lastName
        }
      }
    }
  }
`

const useStyles = createUseStyles((theme: Theme) => ({
  card: {
    borderRadius: '10px',
    borderStyle: 'solid',
    borderWidth: '2px',
    borderColor: theme.color.on.background,
    backgroundColor: theme.color.surface,
    margin: '10px',
    display: 'flex',
    flexDirection: 'column'
  },
  title: {
    display: 'flex',
    flexDirection: 'column',

    '& > a': {
      textDecoration: 'none',
      backgroundColor: theme.color.primary,
      color: theme.color.on.primary,
      fontWeight: 'bold',
      padding: '5px',
      cursor: 'pointer'
    }
  }
}))

/// Parse a GraphQL `Bill` object and render it.
export default function Bill ({ data }: Props): JSX.Element {
  const classes = useStyles()

  const id = data.id
  if (typeof id !== 'number') {
    console.log('Invalid type of id', id)
    return <div>Invalid data</div>
  }

  return <div className={classes.card}>
    <div className={classes.title}>
      <Link to={`/bills/${id}`}>
        {data.state.abbreviation} {data.name} &mdash; {data.title}
      </Link>
    </div>
    <div>
      Summary: {data.summary}
    </div>
  </div>
}
