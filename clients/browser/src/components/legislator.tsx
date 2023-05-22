import React from 'react'
import { gql } from '@apollo/client'
import { type DocumentNode } from 'graphql'
import { createUseStyles } from 'react-jss'
import { Link } from 'react-router-dom'

import { type Theme } from 'themes/theme'

interface Props {
  data: any
}

export const LEGISLATOR_FIELDS: DocumentNode = gql`
  fragment LegislatorFields on Legislator {
    id
    district {
      name
      state { abbreviation }
    }
    party {
      abbreviation
    }
    firstName
    lastName
  }
`

const useStyles = createUseStyles((theme: Theme) => ({
  card: {
    borderRadius: '10px',
    borderStyle: 'solid',
    borderWidth: '2px',
    margin: '10px',
    display: 'flex',
    flexDirection: 'column',
    ...theme.surface({ border: true })
  },
  title: {
    display: 'flex',
    flexDirection: 'column',

    '& > a': {
      textDecoration: 'none',
      fontWeight: 'bold',
      padding: '5px',
      cursor: 'pointer',
      ...theme.primary()
    }
  }
}))

/// Parse a GraphQL `Legislator` object and render it.
export default function Legislator ({ data }: Props): JSX.Element {
  const classes = useStyles()

  const id = data.id
  if (typeof id !== 'number') {
    console.log('Invalid type of id', id)
    return <div>Invalid data</div>
  }

  return <div className={classes.card}>
    <div className={classes.title}>
      <Link to={`/legislators/${id}`}>
        {data.firstName} {data.lastName}
        ({data.party.abbreviation}&ndash;{data.district.state.abbreviation})
      </Link>
    </div>
    <div>
      District: {data.district.name}
    </div>
  </div>
}
