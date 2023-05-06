import React from 'react'
import { Form, Link } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

const useStyles = createUseStyles((theme) => ({
  navBar: {
    display: 'flex',
    justifyContent: 'space-between'
  },
  navItem: {
    margin: '10px 25px'
  },
  searchItem: {
    extend: 'navItem',
    flex: '0 1 700px'
  },
  searchBar: {
    width: '100%',
    padding: '5px',
    margin: '2px',
    borderRadius: '12px',
    fontSize: 24
  }
}))

export default function NavBar (): JSX.Element {
  const classes = useStyles()

  return (
    <nav className={classes.navBar}>
      <div className={classes.navItem}>
        <Link to="/">Logo</Link>
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
