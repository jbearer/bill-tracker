import React from 'react'
import { Link } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

const useStyles = createUseStyles((theme) => ({
  navBar: {
    display: 'flex',
    justifyContent: 'space-between',
    padding: '12px'
  }
}))

export default function NavBar (): JSX.Element {
  const classes = useStyles()
  return (
    <nav className={classes.navBar}>
      <div>
        <Link to="/">Logo</Link>
      </div>
      <div>
        Search
      </div>
      <div>
        Account
      </div>
    </nav>
  )
}
