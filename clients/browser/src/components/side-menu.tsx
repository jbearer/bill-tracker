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
    flexDirection: 'column'
  },
  menuLink: {
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
  },
  footerLink: {
    color: '#828282',
    textDecoration: 'none'
  }
}))

interface MenuLinkProps {
  to: string
  children: any
}

function MenuLink ({ to, children }: MenuLinkProps): JSX.Element {
  const classes = useStyles()
  return (
    <Link className={classes.menuLink} to={to}>
      {children}
    </Link>
  )
}

export default function SideMenu (): JSX.Element {
  const classes = useStyles()
  return (
    <div className={classes.sidebar}>
      <div className={classes.section}>
        <MenuLink to="/"><span>Home</span></MenuLink>
        <MenuLink to="/feed/recent">What&apos;s new?</MenuLink>
        <MenuLink to="/feed/history">History</MenuLink>
        <MenuLink to="/feed/trending">Trending</MenuLink>
      </div>
      <div className={classes.section}>
        <MenuLink to="/issues/1">An issue you might like</MenuLink>
        <MenuLink to="/issues/2">Or how about this issue?</MenuLink>
      </div>
      <div className={classes.section}>
          <Link className={classes.footerLink} to="/license">License</Link>
      </div>
    </div>
  )
}
