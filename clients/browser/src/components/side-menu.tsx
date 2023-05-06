import React from 'react'
import { Link } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

const useStyles = createUseStyles((theme) => ({
  sidebar: {
    display: 'flex',
    flexDirection: 'column',
    margin: '12px',

    '& > :not(:first-child)': {
      // Section dividers.
      borderTop: '1px solid',
      marginTop: '12px',
      paddingTop: '12px'
    }
  },
  section: {
    display: 'flex',
    flexDirection: 'column',

    '& > a': {
      flex: '30px',

      paddingLeft: '12px',
      paddingRight: '12px',
      borderRadius: '5px',
      textDecoration: 'none',
      color: 'black',

      display: 'flex',
      alignItems: 'center',

      '&:hover': {
        backgroundColor: '#f2f2f2'
      }
    }
  },
  header: {
    paddingLeft: '12px',
    paddingRight: '12px'
  },
  footer: {
    display: 'flex',
    flexDirection: 'column',
    paddingLeft: '12px',
    paddingRight: '12px',

    '& > a': {
      color: '#828282',
      textDecoration: 'none'
    }
  }
}))

interface SideMenuSectionProps {
  children: React.ReactNode
}

export function SideMenuSection ({ children }: SideMenuSectionProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.section}>{children}</div>
}

interface SideMenuFooterProps {
  children: React.ReactNode
}

export function SideMenuFooter ({ children }: SideMenuFooterProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.footer}>{children}</div>
}

interface SideMenuHeaderProps {
  children: React.ReactNode
}

export function SideMenuHeader ({ children }: SideMenuHeaderProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.header}><b>{children}</b></div>
}

interface SideMenuLinkProps {
  to: string
  children: React.ReactNode
}

export function SideMenuLink ({ to, children }: SideMenuLinkProps): JSX.Element {
  return <Link to={to}>{children}</Link>
}

interface SideMenuProps {
  children: React.ReactNode
}

export function SideMenu ({ children }: SideMenuProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.sidebar}>{children}</div>
}
