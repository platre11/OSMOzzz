import { NextRequest, NextResponse } from 'next/server'

export function middleware(req: NextRequest) {
  const res = NextResponse.next()
  // Ne pas écraser un choix explicite de l'utilisateur
  if (!req.cookies.get('osmozzz-lang')) {
    const accept = req.headers.get('accept-language') ?? ''
    const detected = accept.slice(0, 2) === 'fr' ? 'fr' : 'en'
    res.cookies.set('osmozzz-lang', detected, { path: '/', sameSite: 'lax' })
  }
  return res
}

export const config = { matcher: '/' }
