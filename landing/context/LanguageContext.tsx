'use client'
import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { translations, Lang, TranslationKey } from '../data/translations'

interface LanguageContextType {
  lang: Lang
  ready: boolean
  setLang: (lang: Lang) => void
  t: (key: TranslationKey) => string
}

const LanguageContext = createContext<LanguageContextType | null>(null)

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>('en')
  const [ready, setReady] = useState(false)

  useEffect(() => {
    const saved = localStorage.getItem('osmozzz-lang') as Lang | null
    const browser = navigator.language.slice(0, 2) as Lang
    const detected =
      saved && ['en', 'fr'].includes(saved)
        ? saved
        : ['en', 'fr'].includes(browser)
        ? browser
        : 'en'
    setLangState(detected)
    setReady(true)
  }, [])

  const setLang = (l: Lang) => {
    setLangState(l)
    localStorage.setItem('osmozzz-lang', l)
  }

  const t = (key: TranslationKey): string => translations[lang][key]

  return (
    <LanguageContext.Provider value={{ lang, ready, setLang, t }}>
      {children}
    </LanguageContext.Provider>
  )
}

export function useLang() {
  const ctx = useContext(LanguageContext)
  if (!ctx) throw new Error('useLang must be used within LanguageProvider')
  return ctx
}
