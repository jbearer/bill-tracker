import React from 'react'

import { SideMenu, SideMenuFooter, SideMenuSection, SideMenuLink } from 'components/side-menu'

export default function AppMenu (): JSX.Element {
  return (
    <SideMenu>
      <SideMenuSection>
        <SideMenuLink to="/"><span>Home</span></SideMenuLink>
        <SideMenuLink to="/feed/recent">What&apos;s new?</SideMenuLink>
        <SideMenuLink to="/feed/history">History</SideMenuLink>
        <SideMenuLink to="/feed/trending">Trending</SideMenuLink>
      </SideMenuSection>
      <SideMenuSection>
        <SideMenuLink to="/issues/1">An issue you might like</SideMenuLink>
        <SideMenuLink to="/issues/2">Or how about this issue?</SideMenuLink>
      </SideMenuSection>
      <SideMenuFooter>
          <SideMenuLink to="/license">License</SideMenuLink>
      </SideMenuFooter>
    </SideMenu>
  )
}
