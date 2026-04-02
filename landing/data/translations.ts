export const translations = {
  en: {
    // Nav
    navGithub: 'GitHub',

    // Hero
    heroInputLabel: 'Your cloud AI',
    heroTypewriter: 'What is Z8xJNaS7f82?',
    heroInputPlaceholder: 'Search your emails, files, notes...',
    heroBadge: 'Open Source · MIT License',
    heroTitle: 'Your AI connected to your world,\nfully under your control.',
    heroSub: 'Connect your AI to everything around you — while staying in control of what it sees and does.',
    heroResponseLabel: 'Response from your AI',
    heroCaptionLine1: 'acts as a customs officer',
    heroCaptionLine2: 'between your sensitive data and cloud AIs.',
    heroCaptionTop: 'OSMOzzz understands Z8xJNaS7f82 - OSMOzzz',
    heroCaptionBottom1: 'OSMOzzz replaces [OSMOzzz] → Z8xJNaS7f82',
    heroDownloadMac: 'Download for Mac',
    heroDownloadWindows: 'Download for Windows',
    heroMeta: 'Free · Open Source',

    // Sources
    sourcesTitle: 'Everything your AI can access',
    sourcesSub: 'You choose what your AI can access. Everything is configurable.',

    // Vision
    visionTitle: 'What is OSMOzzz?',
    visionBody: 'OSMOzzz is a local MCP server that acts as an indispensable security layer between your AI and your sensitive data.\n\nWithout it, every query your AI makes — searching your emails, reading your files — sends your raw data directly to external servers with no filter, no trace, and no control. OSMOzzz intercepts every request: it applies automatic redaction of credit card numbers, IBANs, API keys and personal details, and replaces real identities with aliases before your AI ever sees them.\n\nIn a world where AI models are increasingly connected to everything, OSMOzzz is the only barrier standing between your most sensitive data and the cloud. Nothing leaves your machine unfiltered. Nothing happens without your explicit approval.',

    // Comparison
    compareTitle: 'AI alone vs AI + OSMOzzz',
    compareSub: 'Connecting a tool directly to your AI via MCP works — but at a cost.',
    compareWithoutBadge: 'AI + MCP directly',
    compareWithout1: 'Your API tokens are stored in your AI client config — the provider has access to them',
    compareWithout2: 'Raw data (emails, files) is sent unfiltered to the AI provider servers',
    compareWithout3: 'No control over what sensitive data is transmitted — credit cards, IBANs, passwords go through as-is',
    compareWithout4: 'No intermediate layer between your AI and your tools — actions go through directly',
    compareWithout5: 'No trace of what your AI accessed or did on your behalf',
    compareWithBadge: 'AI + OSMOzzz + MCP',
    compareWith1: 'Your tokens stay in ~/.osmozzz on your machine — never transmitted to the AI provider',
    compareWith2: 'You control exactly what data is transmitted to your AI — not your raw files',
    compareWith3: 'Configure which sensitive data to redact — credit cards, IBANs, API keys, phone numbers',
    compareWith4: 'Configure identity aliases — your AI can work with pseudonyms instead of your real contacts',
    compareWith5: 'Configure an approval layer for actions — and verify any result via cryptographic signature (HMAC-SHA256)',
    compareConclusion: 'OSMOzzz is the firewall between your AI and your data.',

    // CTA
    ctaTitle: 'Give your AI access to your world.',
    ctaSub: 'Open source. You decide what your AI sees.',
    ctaDownloadMac: 'Download for Mac',
    ctaDownloadWindows: 'Download for Windows',
    ctaGithub: 'View on GitHub →',

    // Footer
    footerLicense: 'MIT License',
  },

  fr: {
    // Nav
    navGithub: 'GitHub',

    // Hero
    heroInputLabel: 'Votre IA cloud',
    heroTypewriter: "C'est quoi Z8xJNaS7f82 ?",
    heroInputPlaceholder: 'Chercher vos emails, fichiers, notes...',
    heroBadge: 'Open Source · Licence MIT',
    heroTitle: 'Votre IA connectée à votre monde,\nsous votre contrôle.',
    heroSub: "Connectez votre IA à tout ce qui vous entoure — en gardant le contrôle sur ce qu'elle voit et ce qu'elle fait.",
    heroResponseLabel: 'Réponse de votre IA',
    heroCaptionLine1: 'agit comme un douanier',
    heroCaptionLine2: 'entre vos données sensibles et les IA cloud.',
    heroCaptionTop: 'OSMOzzz comprend Z8xJNaS7f82 - OSMOzzz',
    heroCaptionBottom1: 'OSMOzzz remplace [OSMOzzz] → Z8xJNaS7f82',
    heroDownloadMac: 'Télécharger pour Mac',
    heroDownloadWindows: 'Télécharger pour Windows',
    heroMeta: 'Gratuit · Open Source',

    // Sources
    sourcesTitle: 'Tout ce à quoi votre IA peut accéder',
    sourcesSub: 'Vous choisissez ce à quoi votre IA a accès. Tout est configurable.',

    // Vision
    visionTitle: "C'est quoi OSMOzzz ?",
    visionBody: "OSMOzzz est un serveur MCP local qui agit comme une couche de sécurité indispensable entre votre IA et vos données sensibles.\n\nSans lui, chaque requête de votre IA — recherche dans vos emails, lecture de vos fichiers — envoie vos données brutes vers des serveurs externes sans filtre, sans trace, et sans contrôle. OSMOzzz intercepte chaque requête : il applique un masquage automatique des numéros de CB, IBAN, clés API et informations personnelles, et remplace les vraies identités par des alias avant que votre IA ne les voie.\n\nDans un monde où les modèles d'IA sont de plus en plus connectés à tout, OSMOzzz est le seul rempart entre vos données les plus sensibles et le cloud. Rien ne quitte votre machine sans être filtré. Rien ne se passe sans votre approbation explicite.",

    // Comparison
    compareTitle: 'IA seule vs IA + OSMOzzz',
    compareSub: 'Connecter un outil directement à votre IA via MCP fonctionne — mais à quel prix.',
    compareWithoutBadge: 'IA + MCP directement',
    compareWithout1: "Vos tokens API sont stockés dans la config de votre client IA — le provider y a accès",
    compareWithout2: "Vos données brutes (emails, fichiers) partent non filtrées vers les serveurs du provider",
    compareWithout3: "Aucun contrôle sur ce qui est transmis — CB, IBAN, mots de passe transitent tels quels",
    compareWithout4: "Aucune couche intermédiaire entre votre IA et vos outils — les actions passent directement",
    compareWithout5: "Aucune trace de ce que votre IA a consulté ou fait en votre nom",
    compareWithBadge: 'IA + OSMOzzz + MCP',
    compareWith1: "Vos tokens restent dans ~/.osmozzz sur votre machine — jamais transmis au provider",
    compareWith2: "Vous contrôlez exactement ce qui est transmis à votre IA — pas vos fichiers bruts",
    compareWith3: "Configurez quelles données sensibles masquer — CB, IBAN, clés API, numéros de téléphone",
    compareWith4: "Configurez des alias d'identité — votre IA peut travailler avec des pseudonymes à la place de vos vrais contacts",
    compareWith5: "Configurez une couche d'approbation pour les actions — et vérifiez tout résultat via signature cryptographique (HMAC-SHA256)",
    compareConclusion: "OSMOzzz, c'est le pare-feu entre votre IA et vos données.",

    // CTA
    ctaTitle: 'Donnez à votre IA accès à votre monde.',
    ctaSub: 'Open source. Vous décidez ce que votre IA voit.',
    ctaDownloadMac: 'Télécharger pour Mac',
    ctaDownloadWindows: 'Télécharger pour Windows',
    ctaGithub: 'Voir sur GitHub →',

    // Footer
    footerLicense: 'Licence MIT',
  },
} as const

export type Lang = 'en' | 'fr'
export type TranslationKey = keyof typeof translations.en
