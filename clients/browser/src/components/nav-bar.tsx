import React from 'react'
import { Form, Link } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

import { type Theme } from 'themes/theme'

const useStyles = createUseStyles((theme: Theme) => ({
  navBar: {
    display: 'flex',
    justifyContent: 'space-between',
    ...theme.primaryDark()
  },
  navItem: {
    margin: '10px 25px',
    display: 'flex'
  },
  searchItem: {
    extend: 'navItem',
    flex: '0 1 700px'
  },
  logo: {
    textDecoration: 'none',
    ...theme.secondary()
  },
  searchBar: {
    width: '100%',
    padding: '5px 10px',
    margin: '2px',
    fontSize: 24,
    '&:focus': {
      outline: 'none'
    },
    ...theme.primaryLight({ border: { radius: '12px', width: 0 } }),
    boxShadow: 'inset 0.02em 0.02em 0.01em 0.01em'
  }
}))

export default function NavBar (): JSX.Element {
  const classes = useStyles()

  return (
    <nav className={classes.navBar}>
      <div className={classes.navItem}>
        <Link className={classes.logo} to="/">Logo</Link>
      </div>
      <Form className={classes.searchItem} action="/search">
        <input type="text" name="query" placeholder="Search" className={classes.searchBar}/>
      </Form>
      <div className={classes.navItem}>
        Account
      </div>
    </nav>
  )
}
