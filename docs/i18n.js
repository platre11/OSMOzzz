/**
 * i18n.js — Fichier centralisé de toutes les traductions de la landing page.
 * Pour ajouter une langue : ajouter une clé de langue (ex: 'de') avec toutes les clés existantes.
 * Pour modifier un texte : modifier uniquement ici, jamais dans index.html.
 */

window.I18N = {
  en: {
    // ── Nav ────────────────────────────────────────────────────────────────────
    'nav.github': 'GitHub',

    // ── Hero ───────────────────────────────────────────────────────────────────
    'hero.badge':    'Open Source · MIT License',
    'hero.title':    'Your AI connected to your world,<br/>locked down.',
    'hero.sub':      'Connect your AI to everything around you — while staying in control of what it sees and does.',
    'hero.download': 'Download for macOS',
    'hero.meta':     'macOS 12+ · Free · Open Source',

    // ── Sources ────────────────────────────────────────────────────────────────
    'sources.title':       'Everything your AI can access',
    'sources.sub':         'You choose what your AI can access. Everything is configurable.',
    'sources.local.label': 'Local',
    'sources.cloud.label': 'Cloud',

    // ── Vision ─────────────────────────────────────────────────────────────────
    'vision.title': 'AI should work for you, not around you.',
    'vision.1':     'AI is powerful. But connecting it directly to your tools means handing over your data with no filter, no control, no trace.',
    'vision.2':     'OSMOzzz is the layer that sits between your AI and your world — so you stay in control of what it sees and what it does.',
    'vision.v1.title': 'Control',
    'vision.v1.desc':  'You decide what your AI can access, what gets filtered, and what requires your approval.',
    'vision.v2.title': 'Transparency',
    'vision.v2.desc':  'Every MCP call is logged. Every result is signed. Nothing happens without a trace.',
    'vision.v3.title': 'Open source',
    'vision.v3.desc':  'The code is public. Verify it, fork it, run it yourself.',

    // ── Comparison ─────────────────────────────────────────────────────────────
    'compare.title': 'AI alone vs AI + OSMOzzz',
    'compare.sub':   'Connecting a tool directly to your AI via MCP works — but at a cost.',

    'compare.without.badge': 'AI + MCP directly',
    'compare.without.1': 'Your API tokens are stored in your AI client config — the provider has access to them',
    'compare.without.2': 'Raw data (emails, files, messages) is sent unfiltered to the AI provider servers',
    'compare.without.3': 'No control over what sensitive data is transmitted — credit cards, IBANs, passwords go through as-is',
    'compare.without.4': 'No intermediate layer between your AI and your tools — actions go through directly',
    'compare.without.5': 'No trace of what your AI accessed or did on your behalf',

    'compare.with.badge': 'AI + OSMOzzz + MCP',
    'compare.with.1': 'Your tokens stay in ~/.osmozzz on your machine — never transmitted to the AI provider',
    'compare.with.2': 'You control exactly what data is transmitted to your AI — not your raw files',
    'compare.with.3': 'Configure which sensitive data to redact — credit cards, IBANs, API keys, phone numbers',
    'compare.with.4': 'Configure identity aliases — your AI can work with pseudonyms instead of your real contacts',
    'compare.with.5': 'Configure an approval layer for actions — and verify any result via cryptographic signature (HMAC-SHA256)',

    'compare.conclusion': 'OSMOzzz is the firewall between your AI and your data.',

    // ── CTA ────────────────────────────────────────────────────────────────────
    'cta.title':    'Give your AI access to your world.',
    'cta.sub':      'Open source. You decide what your AI sees.',
    'cta.download': 'Download for macOS',
    'cta.github':   'View on GitHub →',

    // ── Footer ─────────────────────────────────────────────────────────────────
    'footer.license': 'MIT License',
  },

  fr: {
    // ── Nav ────────────────────────────────────────────────────────────────────
    'nav.github': 'GitHub',

    // ── Hero ───────────────────────────────────────────────────────────────────
    'hero.badge':    'Open Source · Licence MIT',
    'hero.title':    'Votre IA connectée à votre monde,<br/>mais sécurisée.',
    'hero.sub':      'Connectez votre IA à tout ce qui vous entoure — en gardant le contrôle sur ce qu\'elle voit et ce qu\'elle fait.',
    'hero.download': 'Télécharger pour macOS',
    'hero.meta':     'macOS 12+ · Gratuit · Open Source',

    // ── Sources ────────────────────────────────────────────────────────────────
    'sources.title':       'Tout ce à quoi votre IA peut accéder',
    'sources.sub':         'Vous choisissez ce à quoi votre IA a accès. Tout est configurable.',
    'sources.local.label': 'Local',
    'sources.cloud.label': 'Cloud',

    // ── Vision ─────────────────────────────────────────────────────────────────
    'vision.title': 'L\'IA devrait travailler pour vous, pas autour de vous.',
    'vision.1':     'L\'IA est puissante. Mais la connecter directement à vos outils signifie lui confier vos données sans filtre, sans contrôle, sans trace.',
    'vision.2':     'OSMOzzz est la couche qui s\'interpose entre votre IA et votre monde — pour que vous gardiez le contrôle sur ce qu\'elle voit et ce qu\'elle fait.',
    'vision.v1.title': 'Contrôle',
    'vision.v1.desc':  'Vous décidez ce à quoi votre IA peut accéder, ce qui est filtré, et ce qui nécessite votre accord.',
    'vision.v2.title': 'Transparence',
    'vision.v2.desc':  'Chaque appel MCP est journalisé. Chaque résultat est signé. Rien ne se passe sans trace.',
    'vision.v3.title': 'Open source',
    'vision.v3.desc':  'Le code est public. Vérifiez-le, forkez-le, faites-le tourner vous-même.',

    // ── Comparison ─────────────────────────────────────────────────────────────
    'compare.title': 'IA seule vs IA + OSMOzzz',
    'compare.sub':   'Connecter un outil directement à votre IA via MCP fonctionne — mais à quel prix.',

    'compare.without.badge': 'IA + MCP directement',
    'compare.without.1': 'Vos tokens API sont stockés dans la config de votre client IA — le provider y a accès',
    'compare.without.2': 'Vos données brutes (emails, fichiers, messages) partent non filtrées vers les serveurs du provider',
    'compare.without.3': 'Aucun contrôle sur ce qui est transmis — CB, IBAN, mots de passe transitent tels quels',
    'compare.without.4': 'Aucune couche intermédiaire entre votre IA et vos outils — les actions passent directement',
    'compare.without.5': 'Aucune trace de ce que votre IA a consulté ou fait en votre nom',

    'compare.with.badge': 'IA + OSMOzzz + MCP',
    'compare.with.1': 'Vos tokens restent dans ~/.osmozzz sur votre machine — jamais transmis au provider',
    'compare.with.2': 'Vous contrôlez exactement ce qui est transmis à votre IA — pas vos fichiers bruts',
    'compare.with.3': 'Configurez quelles données sensibles masquer — CB, IBAN, clés API, numéros de téléphone',
    'compare.with.4': 'Configurez des alias d\'identité — votre IA peut travailler avec des pseudonymes à la place de vos vrais contacts',
    'compare.with.5': 'Configurez une couche d\'approbation pour les actions — et vérifiez tout résultat via signature cryptographique (HMAC-SHA256)',

    'compare.conclusion': 'OSMOzzz, c\'est le pare-feu entre votre IA et vos données.',

    // ── CTA ────────────────────────────────────────────────────────────────────
    'cta.title':    'Donnez à votre IA accès à votre monde.',
    'cta.sub':      'Open source. Vous décidez ce que votre IA voit.',
    'cta.download': 'Télécharger pour macOS',
    'cta.github':   'Voir sur GitHub →',

    // ── Footer ─────────────────────────────────────────────────────────────────
    'footer.license': 'Licence MIT',
  },
}
