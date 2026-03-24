export const translations = {
  en: {
    // Nav
    navGithub: 'GitHub',

    // Hero
    heroInputLabel: 'Your cloud AI',
    heroTypewriter: 'What is Z8xJNaS7f82?',
    heroInputPlaceholder: 'Search your emails, files, notes...',
    heroBadge: 'Open Source · MIT License',
    heroTitle: 'Your AI connected to your world,\nlocked down.',
    heroSub: 'Connect your AI to everything around you — while staying in control of what it sees and does.',
    heroResponseLabel: 'Response from your AI',
    heroCaptionLine1: 'acts as a customs officer',
    heroCaptionLine2: 'between your sensitive data and cloud AIs.',
    heroCaptionTop: 'OSMOzzz understands Z8xJNaS7f82 - OSMOzzz',
    heroCaptionBottom1: 'OSMOzzz replaces [OSMOzzz] → Z8xJNaS7f82',
    heroDownload: 'Download for macOS',
    heroMeta: 'macOS 12+ · Free · Open Source',

    // Sources
    sourcesTitle: 'Everything your AI can access',
    sourcesSub: 'You choose what your AI can access. Everything is configurable.',
    sourcesLocalLabel: 'Local',
    sourcesCloudLabel: 'Cloud',

    // Vision
    visionTitle: 'What is OSMOzzz?',
    visionLead: 'OSMOzzz is a local MCP server that connects your AI to your data — emails, files, messages, notes, calendar, and cloud tools. It acts as a privacy layer between your AI and your world: you stay in control of what it sees and what it does.',
    visionF1Title: 'Sensitive data filtering',
    visionF1Desc: 'Configure automatic redaction of credit card numbers, IBANs, API keys, or phone numbers before anything reaches your AI.',
    visionF2Title: 'Identity aliases',
    visionF2Desc: "Replace real names and email addresses with aliases. Your AI works with pseudonyms — it never sees your actual contacts unless you allow it.",
    visionF3Title: 'Blacklist',
    visionF3Desc: "Exclude specific documents, senders, domains, or file paths from your AI's reach. Blocked at indexing time, not just at search time.",
    visionF4Title: 'Action approval',
    visionF4Desc: 'When your AI wants to send a message, create a task, or modify a file — you approve or reject each action before it runs.',
    visionF5Title: 'Audit log',
    visionF5Desc: 'Every MCP call is logged locally: which tool, which query, how many results, blocked or not. Nothing happens without a trace.',

    // Comparison
    compareTitle: 'AI alone vs AI + OSMOzzz',
    compareSub: 'Connecting a tool directly to your AI via MCP works — but at a cost.',
    compareWithoutBadge: 'AI + MCP directly',
    compareWithout1: 'Your API tokens are stored in your AI client config — the provider has access to them',
    compareWithout2: 'Raw data (emails, files, messages) is sent unfiltered to the AI provider servers',
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
    ctaDownload: 'Download for macOS',
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
    heroTitle: 'Votre IA connectée à votre monde,\nmais sécurisée.',
    heroSub: "Connectez votre IA à tout ce qui vous entoure — en gardant le contrôle sur ce qu'elle voit et ce qu'elle fait.",
    heroResponseLabel: 'Réponse de votre IA',
    heroCaptionLine1: 'agit comme un douanier',
    heroCaptionLine2: 'entre vos données sensibles et les IA cloud.',
    heroCaptionTop: 'OSMOzzz comprend Z8xJNaS7f82 - OSMOzzz',
    heroCaptionBottom1: 'OSMOzzz remplace [OSMOzzz] → Z8xJNaS7f82',
    heroDownload: 'Télécharger pour macOS',
    heroMeta: 'macOS 12+ · Gratuit · Open Source',

    // Sources
    sourcesTitle: 'Tout ce à quoi votre IA peut accéder',
    sourcesSub: 'Vous choisissez ce à quoi votre IA a accès. Tout est configurable.',
    sourcesLocalLabel: 'Local',
    sourcesCloudLabel: 'Cloud',

    // Vision
    visionTitle: "C'est quoi OSMOzzz ?",
    visionLead: "OSMOzzz est un serveur MCP local qui connecte votre IA à vos données — emails, fichiers, messages, notes, agenda, et outils cloud. Il agit comme une couche de confidentialité entre votre IA et votre monde : vous gardez le contrôle sur ce qu'elle voit et ce qu'elle fait.",
    visionF1Title: 'Filtrage des données sensibles',
    visionF1Desc: "Configurez le masquage automatique des numéros de CB, IBAN, clés API ou numéros de téléphone avant qu'ils n'atteignent votre IA.",
    visionF2Title: "Alias d'identité",
    visionF2Desc: "Remplacez les vrais noms et adresses email par des alias. Votre IA travaille avec des pseudonymes — elle ne voit jamais vos vrais contacts, sauf si vous l'autorisez.",
    visionF3Title: 'Liste noire',
    visionF3Desc: "Excluez des documents, expéditeurs, domaines ou chemins de fichiers de la portée de votre IA. Bloqués à l'indexation, pas seulement à la recherche.",
    visionF4Title: 'Approbation des actions',
    visionF4Desc: "Quand votre IA veut envoyer un message, créer une tâche ou modifier un fichier — vous approuvez ou rejetez chaque action avant qu'elle ne s'exécute.",
    visionF5Title: "Journal d'accès",
    visionF5Desc: "Chaque appel MCP est journalisé localement : quel outil, quelle requête, combien de résultats, bloqué ou non. Rien ne se passe sans trace.",

    // Comparison
    compareTitle: 'IA seule vs IA + OSMOzzz',
    compareSub: 'Connecter un outil directement à votre IA via MCP fonctionne — mais à quel prix.',
    compareWithoutBadge: 'IA + MCP directement',
    compareWithout1: "Vos tokens API sont stockés dans la config de votre client IA — le provider y a accès",
    compareWithout2: "Vos données brutes (emails, fichiers, messages) partent non filtrées vers les serveurs du provider",
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
    ctaDownload: 'Télécharger pour macOS',
    ctaGithub: 'Voir sur GitHub →',

    // Footer
    footerLicense: 'Licence MIT',
  },
} as const

export type Lang = 'en' | 'fr'
export type TranslationKey = keyof typeof translations.en
