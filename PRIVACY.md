# Politique de confidentialité — Popdoc

*Dernière mise à jour : 11 juin 2026*

> La version courte : **ce qui se passe sur ton Mac reste sur ton Mac.** La
> gratuité ne se paie pas en données. Voici exactement ce que Popdoc voit,
> touche et garde — c'est-à-dire presque rien.

Popdoc est un utilitaire macOS gratuit et open source édité par **Guillaume
Cyr** (« je », « nous »). Il importe tes fichiers Office dans ton propre Google
Drive et les convertit en documents Google natifs. Cette page explique
comment il traite tes données.

## Qui est responsable

Guillaume Cyr — contact : **guillaume@getpopdoc.com**. Popdoc n'opère **aucun
serveur** : il n'y a personne d'autre dans la boucle que toi et Google.

## Ce que Popdoc touche

Pour faire son travail, Popdoc a besoin d'un accès à ton Google Drive, demandé
au moment de la connexion via l'écran de consentement officiel de Google :

- **Le périmètre `drive.file` uniquement.** C'est le périmètre le plus
  restreint possible : Popdoc ne peut voir et toucher **que les fichiers qu'il
  crée lui-même** dans ton Drive. Il ne peut pas lire le reste de ton Drive.
- **Le contenu des fichiers que tu lui donnes à ouvrir.** Quand tu
  double-cliques un fichier, son contenu est envoyé **directement de ton Mac
  vers l'API de Google Drive** pour y être converti. Il ne passe par aucun
  serveur intermédiaire.
- **Des jetons d'authentification (tokens OAuth).** Ils prouvent à Google que
  tu as autorisé Popdoc.

## Ce que Popdoc garde — et où

- **Tes jetons d'accès** sont rangés dans le **Trousseau d'accès de macOS** — le
  coffre-fort de mots de passe intégré à ton Mac, chiffré et protégé par ta
  session. Ils restent **uniquement sur ta machine** et n'en sortent jamais, sauf
  pour parler à Google. *(Détail technique : trousseau « login », service
  `Popdoc`, compte `Google Drive`.)*
- **Un index local de déduplication** (`imports.json` dans
  `~/Library/Application Support/Popdoc/` + attributs étendus sur tes fichiers)
  sert uniquement à éviter de recréer un doublon quand tu rouvres un fichier déjà
  importé. Il reste sur ton Mac.

## Ce que Popdoc ne fait PAS

- ❌ Aucune donnée personnelle n'est **collectée**, requise ou stockée par nous.
- ❌ Aucun **tracking**, aucune **publicité**, aucun **analytics**, aucun cookie.
- ❌ Aucune donnée n'est **vendue, partagée ou transmise** à un tiers — il n'y a
  ni serveur, ni base de données, ni destinataire de notre côté.
- ❌ Popdoc ne lit **pas** le reste de ton Google Drive.

## Limited Use (conformité Google API)

L'utilisation et le transfert par Popdoc des informations reçues des API Google
respectent la
[Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy),
y compris ses exigences **Limited Use**. Popdoc utilise l'accès `drive.file`
strictement pour importer et convertir les fichiers que tu lui confies, et pour
rien d'autre.

## Supprimer tes données

Comme tout reste en local, tu gardes le contrôle :

- **Révoquer l'accès** : depuis ton
  [compte Google → Sécurité → Applications tierces](https://myaccount.google.com/connections).
- **Effacer les jetons locaux** : ouvre l'app **Trousseau d'accès**, cherche
  `Popdoc` et supprime l'entrée. Supprime aussi le dossier
  `~/Library/Application Support/Popdoc/` pour effacer l'index de déduplication.
- Les fichiers déjà convertis t'appartiennent : ils sont dans **ton** Drive, tu
  les gères comme n'importe quel document Google.

## Modifications

Si cette politique change, la date en haut de page sera mise à jour et la
nouvelle version sera publiée à la même adresse. Pas de mauvaise surprise.

## Contact

Une question sur tes données ? Écris à **guillaume@getpopdoc.com**.

---

# Privacy Policy — Popdoc

*Last updated: June 11, 2026*

> The short version: **what happens on your Mac stays on your Mac.** Free
> doesn't mean paying with your data. Here is exactly what Popdoc sees, touches
> and keeps — which is almost nothing.

Popdoc is a free, open-source macOS utility made by **Guillaume Cyr** ("I",
"we"). It imports your Office files into your own Google Drive and converts them
into native Google documents. This page explains how it handles your data.

## Who is responsible

Guillaume Cyr — contact: **guillaume@getpopdoc.com**. Popdoc runs **no servers**:
there is no one in the loop but you and Google.

## What Popdoc touches

To do its job, Popdoc needs access to your Google Drive, requested at sign-in
through Google's official consent screen:

- **The `drive.file` scope only.** This is the narrowest possible scope: Popdoc
  can only see and touch **the files it creates itself** in your Drive. It
  cannot read the rest of your Drive.
- **The content of the files you ask it to open.** When you double-click a file,
  its content is sent **directly from your Mac to the Google Drive API** to be
  converted. It does not pass through any intermediary server.
- **Authentication tokens (OAuth tokens).** They prove to Google that you
  authorized Popdoc.

## What Popdoc keeps — and where

- **Your access tokens** live in the **macOS Keychain** — your Mac's built-in,
  encrypted password vault, protected by your login session. They stay **only on
  your machine** and never leave it except to talk to Google. *(Technical detail:
  "login" keychain, service `Popdoc`, account `Google Drive`.)*
- **A local dedup index** (`imports.json` in
  `~/Library/Application Support/Popdoc/` + extended attributes on your files)
  exists only to avoid creating a duplicate when you re-open an
  already-imported file. It stays on your Mac.

## What Popdoc does NOT do

- ❌ No personal data is **collected**, required or stored by us.
- ❌ No **tracking**, no **ads**, no **analytics**, no cookies.
- ❌ No data is **sold, shared or transferred** to any third party — there is no
  server, no database, and no recipient on our side.
- ❌ Popdoc does **not** read the rest of your Google Drive.

## Limited Use (Google API compliance)

Popdoc's use and transfer of information received from Google APIs adheres to
the
[Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy),
including its **Limited Use** requirements. Popdoc uses `drive.file` access
strictly to import and convert the files you hand it, and for nothing else.

## Deleting your data

Since everything stays local, you stay in control:

- **Revoke access**: from your
  [Google Account → Security → Third-party apps](https://myaccount.google.com/connections).
- **Erase local tokens**: open the **Keychain Access** app, search for `Popdoc`
  and delete the entry. Also delete the
  `~/Library/Application Support/Popdoc/` folder to clear the dedup index.
- Already-converted files are yours: they live in **your** Drive, and you manage
  them like any other Google document.

## Changes

If this policy changes, the date at the top will be updated and the new version
published at the same address. No nasty surprises.

## Contact

A question about your data? Write to **guillaume@getpopdoc.com**.
