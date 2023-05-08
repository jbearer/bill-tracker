import React from 'react'
import { gql } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { createUseStyles } from 'react-jss'
import { Link } from 'react-router-dom'

import { type Theme } from 'themes/theme'

interface Props {
  data: any
}

export const ISSUE_FIELDS: DocumentNode = gql`
  fragment IssueFields on Issue {
    id
    name
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

/// Parse a GraphQL `Issue` object and render it.
export default function Issue ({ data }: Props): JSX.Element {
  const classes = useStyles()

  const id = data.id
  if (typeof id !== 'number') {
    console.log('Invalid type of id', id)
    return <div>Invalid data</div>
  }

  return <div className={classes.card}>
    <div className={classes.title}>
      <Link to={`/issues/${id}`}>
        {data.name} (issue)
      </Link>
    </div>
    <div>
    </div>
  </div>
}
