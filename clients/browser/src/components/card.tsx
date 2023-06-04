import React from 'react'
import { createUseStyles } from 'react-jss'

import { type Theme } from 'themes/theme'

const useCardStyles = createUseStyles((theme: Theme) => ({
  card: {
    margin: '10px',
    display: 'flex',
    flexDirection: 'column',
    ...theme.surface({ border: { radius: '10px' } })
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
  },
  content: {
    padding: '5px'
  }
}))

interface CardProps {
  children: React.ReactNode
}

/// Display a brief summary of an entity as a card.
export function Card ({ children }: CardProps): JSX.Element {
  const classes = useCardStyles()
  return <div className={classes.card}>
    {children}
  </div>
}

/// The title section of a `Card`.
export function Title ({ children }: CardProps): JSX.Element {
  const classes = useCardStyles()
  return <div className={classes.title}>
    {children}
  </div>
}

/// The body section of a `Card`.
export function Body ({ children }: CardProps): JSX.Element {
  const classes = useCardStyles()
  return <div className={classes.content}>
    {children}
  </div>
}
