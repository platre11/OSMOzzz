/**
 * lang.js — Moteur de traduction.
 * Lit les clés data-i18n dans le DOM et les remplace par les textes de I18N.
 * Ne pas modifier pour changer un texte — utiliser i18n.js.
 */

(function () {
  const SUPPORTED = ['en', 'fr']
  const DEFAULT   = 'en'

  function detectLang() {
    const saved   = localStorage.getItem('osmozzz-lang')
    const browser = navigator.language.slice(0, 2)
    if (saved && SUPPORTED.includes(saved))   return saved
    if (SUPPORTED.includes(browser))          return browser
    return DEFAULT
  }

  function applyLang(lang) {
    const dict = window.I18N[lang]
    if (!dict) return

    document.querySelectorAll('[data-i18n]').forEach(el => {
      const key = el.getAttribute('data-i18n')
      if (dict[key] !== undefined) el.innerHTML = dict[key]
    })

    // Boutons switcher
    document.querySelectorAll('.lang-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.lang === lang)
    })

    document.documentElement.lang = lang
    localStorage.setItem('osmozzz-lang', lang)
  }

  // Exposé globalement pour les boutons onclick
  window.setLang = applyLang

  // Init au chargement
  document.addEventListener('DOMContentLoaded', () => applyLang(detectLang()))
})()
