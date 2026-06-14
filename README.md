# Popdoc

Double-clique sur n'importe quel fichier Office (`.docx`, `.xlsx`, `.pptx`, `.doc`, `.xls`, `.ppt`, `.odt`, `.rtf`, `.csv`) sur macOS : il est automatiquement importé + converti dans ton Google Drive (Docs / Sheets / Slides) et ouvert dans ton navigateur.

Une petite fenêtre flottante (fade in / spinner / checkmark / fade out) affiche la progression. Si le fichier a déjà été importé, un dialog natif te propose **Ouvrir l'existant** ou **Importer une copie**.

App Tauri (Rust). ~4.5 MB installé. Aucun serveur tiers.

---

## Pour les utilisateurs

1. Télécharge `Popdoc.dmg` depuis Releases.
2. Glisse l'app dans `/Applications`.
3. **Premier lancement** (l'app n'est pas signée Apple Developer ID) :
   - Clic droit sur l'app → **Ouvrir** → confirmer
   - Ou : `xattr -dr com.apple.quarantine "/Applications/Popdoc.app"`
4. Dans Finder, clic droit sur un `.docx` → **Ouvrir avec → Popdoc** (coche "Toujours ouvrir avec" pour associer définitivement).
5. Au premier import : une fenêtre OAuth s'ouvre dans ton navigateur, autorise. Les tokens sont stockés dans `~/Library/Application Support/Popdoc/tokens.json` (`0600`).

À partir de là : double-clic = import automatique + ouverture dans le navigateur.

**Note sur les fichiers téléchargés** : macOS marque les fichiers d'Internet comme "quarantaine" et peut bloquer leur ouverture par une app non-signée Apple. Si tu rencontres un warning :
```bash
xattr -dr com.apple.quarantine /chemin/vers/le-fichier.docx
```

---

## Pour le développeur

### 1. Setup Google Cloud (une seule fois pour tous tes utilisateurs)

1. https://console.cloud.google.com/ → nouveau projet `Popdoc`
2. **APIs & Services → Library** → active **Google Drive API**
3. **Google Auth Platform → Branding/Audience** :
   - User Type : **External**
   - Scope : `.../auth/drive.file`
   - Audience : passe en **Production** (le scope `drive.file` est non-sensitive → pas de vérification Google requise → n'importe qui peut se connecter)
4. **Google Auth Platform → Clients → Créer un client** :
   - Type : **Application de bureau**
   - Note le **Client ID** et le **Client Secret**

Le `client_secret` d'une "Desktop app" n'est officiellement pas confidentiel ([doc Google](https://developers.google.com/identity/protocols/oauth2#installed)) — c'est OK de l'embarquer dans l'app distribuée.

### 2. Configurer les credentials

```bash
cp .env.example .env
# Édite .env avec ton Client ID + Client Secret
```

Le fichier `.env` est gitignored. Il est lu par `src-tauri/build.rs` au moment du build et les valeurs sont injectées dans le binaire via `env!()`. Aucun secret n'est commité.

### 3. Build

```bash
npm install
npm run tauri build
```

Le `.dmg` est dans `src-tauri/target/release/bundle/dmg/`. L'app est ad-hoc signée automatiquement (`signingIdentity: "-"` dans `tauri.conf.json`), ce qui permet l'association de fichiers "Always Open With…".

Pour générer les icônes à partir d'un PNG 1024×1024 :
```bash
npm run tauri icon /chemin/vers/icone.png
```

---

## Architecture

```
Popdoc/
├── .env                                ← gitignored, contient les vraies clés
├── .env.example                        ← template tracké
├── package.json                        ← @tauri-apps/cli uniquement
├── src/                                ← UI
│   └── progress.html                   ← fenêtre flottante fade in/out
└── src-tauri/
    ├── tauri.conf.json                 ← file associations, bundle, ad-hoc signing
    ├── capabilities/default.json       ← permissions plugins
    ├── icons/                          ← générées par `tauri icon`
    ├── build.rs                        ← charge .env, expose via cargo:rustc-env
    └── src/
        ├── lib.rs                      ← orchestration, queue, fenêtre progress
        ├── credentials.rs              ← env!("GDL_CLIENT_ID") / env!("GDL_CLIENT_SECRET")
        ├── config.rs                   ← constantes, chemins app data
        ├── oauth.rs                    ← PKCE loopback flow + storage fichier
        ├── drive.rs                    ← appels Drive API (reqwest)
        ├── dedup.rs                    ← xattr + SHA-256 + JSON local
        └── mime_map.rs                 ← extensions → MIME Google Docs
```

## Comment ça marche

- **OAuth 2.0 PKCE Desktop App** — flow loopback `127.0.0.1`, pas de serveur tiers.
- **Scope `drive.file`** — l'app n'accède qu'aux fichiers qu'elle crée.
- **Tokens** stockés en `~/Library/Application Support/Popdoc/tokens.json` (`0600`, modèle gcloud / aws CLI).
- **Détection de doublon** : xattr `com.gdoclauncher.drive_id` sur le fichier local → fallback hash SHA-256 dans `imports.json`.
- **Dossier de destination** : `Imports Office` à la racine de Mon Drive, créé au premier import.
- **Fenêtre de progression** : webview Tauri transparent + always-on-top, fade in/out CSS, status mis à jour par events Rust.

## Limitations connues

- **Gatekeeper** : ad-hoc signing résout le problème de "toujours ouvrir avec…", mais les fichiers téléchargés (xattr `com.apple.quarantine`) peuvent encore être bloqués au premier coup. Apple Developer ID ($99/an) éliminerait ça complètement.
- **xattr perdu** sur certains transferts (email, FAT, zip/unzip) — d'où le fallback SHA-256.

---

## Licence

Ce projet utilise une **licence scindée** — le code est ouvert, la marque ne l'est pas :

| Élément | Licence |
|---------|---------|
| **Code source** | [Apache License 2.0](./LICENSE) — usage, modification, redistribution libres |
| **Nom « Popdoc » & logo** | Réservés — voir [TRADEMARK.md](./TRADEMARK.md) |
| **Icônes, mascotte, identité visuelle (DA)** | Tous droits réservés — voir [LICENSE-ASSETS.md](./LICENSE-ASSETS.md) |

En clair : tu peux librement t'appuyer sur le **code**, mais si tu distribues un
fork tu dois le **rebrander** — choisis ton propre nom et tes propres icônes. Le
nom Popdoc, le logo et la direction artistique restent la propriété exclusive de
**Guillaume Cyr**.

## Confidentialité & conditions

- [Politique de confidentialité](./PRIVACY.md) — *« ce qui se passe sur ton Mac reste sur ton Mac »* (FR + EN)
- [Conditions d'utilisation](./TERMS.md) — usage as-is, sans garantie (FR + EN)

© 2026 Guillaume Cyr.
