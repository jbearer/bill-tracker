import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import reportWebVitals from './reportWebVitals'
import {
  createBrowserRouter,
  createRoutesFromElements,
  Route,
  RouterProvider
} from 'react-router-dom'
import { ThemeProvider } from 'react-jss'
import { ApolloClient, InMemoryCache, ApolloProvider } from '@apollo/client'
import { relayStylePagination } from '@apollo/client/utilities'

import defaultTheme from 'themes/default'

import Bill from 'views/Bill'
import Error from 'views/Error'
import Feed, { FeedType } from 'views/Feed'
import Issue from 'views/Issue'
import Legislator from 'views/Legislator'
import License from 'views/License'
import Search, { SearchType } from 'views/Search'

const apolloClient = new ApolloClient({
  uri: process.env.REACT_APP_BILL_TRACKER_SERVER_URL,
  cache: new InMemoryCache({
    typePolicies: {
      Query: {
        fields: {
          bills: relayStylePagination(['where']),
          legislators: relayStylePagination(['where']),
          issues: relayStylePagination(['where'])
        }
      }
    }
  })
})

const router = createBrowserRouter(
  createRoutesFromElements(<Route path = "/" element={<App />} errorElement={<Error />}>
    <Route path="/" element={<Feed />} />

    <Route path="/feed/recent" element={<Feed key="recent" type={FeedType.Recent}/>} />
    <Route path="/feed/trending" element={<Feed key="trending" type={FeedType.Trending}/>} />
    <Route path="/feed/history" element={<Feed key="history" type={FeedType.History}/>} />

    <Route path="/search" element={<Search key="all" type={SearchType.All}/>} />
    <Route path="/search/bills" element={<Search key="bills" type={SearchType.Bills}/>} />
    <Route path="/search/people" element={<Search key="people" type={SearchType.People}/>} />
    <Route path="/search/issues" element={<Search key="issues" type={SearchType.Issues}/>} />

    <Route path="/bills/:id" element={<Bill />} />
    <Route path="/legislators/:id" element={<Legislator />} />
    <Route path="/issues/:id" element={<Issue />} />
    <Route path="/license" element={<License />} />
  </Route>)
)

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
)
root.render(
  <React.StrictMode>
    <ApolloProvider client={apolloClient}>
      <ThemeProvider theme={defaultTheme}>
        <RouterProvider router={router} />
      </ThemeProvider>
    </ApolloProvider>
  </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
