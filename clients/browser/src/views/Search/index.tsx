import React from 'react'
import { useSearchParams } from 'react-router-dom'

import { SideMenu, SideMenuSection, SideMenuLink, SideMenuHeader } from 'components/side-menu'
import MainLayout from 'layouts/main'

export enum SearchType {
  All,
  Bills,
  People,
  Issues,
}

interface SearchProps {
  type: SearchType
}

export default function Search ({ type }: SearchProps): JSX.Element {
  const params = useSearchParams()[0]
  const query = params.get('query') ?? ''

  const menu =
    <SideMenu>
      <SideMenuSection>
        <SideMenuHeader>I&apos;m looking for...</SideMenuHeader>
        <SideMenuLink to={`/search/bills?query=${query}`}>Bills</SideMenuLink>
        <SideMenuLink to={`/search/issues?query=${query}`}>Issues</SideMenuLink>
        <SideMenuLink to={`/search/people?query=${query}`}>People</SideMenuLink>
      </SideMenuSection>
    </SideMenu>

  return (
    <MainLayout menu={menu}>
      {type} results for {query}
    </MainLayout>
  )
}
