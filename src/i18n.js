// Shared translation table for the web UIs (progress.html, settings.html).
// Mirrors the native table in src-tauri/src/i18n.rs — keep them in sync.
// Languages: fr / en / es (same three the website ships in).
//
// Usage:
//   t(lang, key)            -> string for a single key
//   applyTranslations(lang) -> fills every [data-i18n] textContent and
//                              [data-i18n-ph] placeholder in the document
// `lang` is one of "fr" | "en" | "es"; anything else falls back to "en".

(function () {
  const STRINGS = {
    fr: {
      "settings.subtitle": "Réglages",
      "settings.folderLabel": "Dossier de destination dans Google Drive",
      "settings.folderPlaceholder": "Imports Office",
      "settings.save": "Enregistrer",
      "settings.folderHelp":
        "Tous les fichiers importés seront placés dans ce dossier à la racine de ton Drive. Le dossier sera créé automatiquement au prochain import s'il n'existe pas.",
      "settings.account": "Compte Google",
      "settings.signout": "Se déconnecter",
      "settings.accountHelp":
        "Supprime les tokens locaux. Le prochain import demandera à nouveau l'autorisation Google.",
      "settings.languageLabel": "Langue",
      "settings.langAuto": "Système",
      "settings.close": "Fermer",
      "settings.saved": "Enregistré",
      "settings.errorPrefix": "Erreur : ",
      "settings.signoutConfirm":
        "Se déconnecter de Google ? Tu devras réautoriser au prochain import.",
      "settings.signedOut": "Déconnecté",
      "settings.updates": "Mises à jour",
      "settings.checkUpdates": "Vérifier les mises à jour",
      "settings.checking": "Vérification…",
      "settings.upToDate": "Popdoc est à jour.",
      "settings.updateDownloaded": "Mise à jour {v} prête — elle s'installera à la fermeture.",
      "settings.updateError": "Échec de la vérification. Réessaie plus tard.",
      "settings.updatesHelp":
        "Popdoc vérifie discrètement les mises à jour au lancement et les installe à la fermeture, sans interrompre l'ouverture de tes fichiers.",
      "settings.version": "Version",
      "progress.importing": "Import en cours…",
      "progress.opening": "Ouverture dans Google Docs…",
      "progress.error": "Erreur",
    },
    en: {
      "settings.subtitle": "Settings",
      "settings.folderLabel": "Destination folder in Google Drive",
      "settings.folderPlaceholder": "Office Imports",
      "settings.save": "Save",
      "settings.folderHelp":
        "All imported files are placed in this folder at the root of your Drive. The folder is created automatically on the next import if it doesn't exist.",
      "settings.account": "Google account",
      "settings.signout": "Sign out",
      "settings.accountHelp":
        "Removes the local tokens. The next import will ask for Google authorization again.",
      "settings.languageLabel": "Language",
      "settings.langAuto": "System",
      "settings.close": "Close",
      "settings.saved": "Saved",
      "settings.errorPrefix": "Error: ",
      "settings.signoutConfirm":
        "Sign out of Google? You'll need to authorize again on the next import.",
      "settings.signedOut": "Signed out",
      "settings.updates": "Updates",
      "settings.checkUpdates": "Check for updates",
      "settings.checking": "Checking…",
      "settings.upToDate": "Popdoc is up to date.",
      "settings.updateDownloaded": "Update {v} ready — it will install on quit.",
      "settings.updateError": "Check failed. Try again later.",
      "settings.updatesHelp":
        "Popdoc quietly checks for updates at launch and installs them on quit, without interrupting opening your files.",
      "settings.version": "Version",
      "progress.importing": "Importing…",
      "progress.opening": "Opening in Google Docs…",
      "progress.error": "Error",
    },
    es: {
      "settings.subtitle": "Ajustes",
      "settings.folderLabel": "Carpeta de destino en Google Drive",
      "settings.folderPlaceholder": "Importaciones de Office",
      "settings.save": "Guardar",
      "settings.folderHelp":
        "Todos los archivos importados se colocan en esta carpeta en la raíz de tu Drive. La carpeta se crea automáticamente en la próxima importación si no existe.",
      "settings.account": "Cuenta de Google",
      "settings.signout": "Cerrar sesión",
      "settings.accountHelp":
        "Elimina los tokens locales. La próxima importación volverá a solicitar la autorización de Google.",
      "settings.languageLabel": "Idioma",
      "settings.langAuto": "Sistema",
      "settings.close": "Cerrar",
      "settings.saved": "Guardado",
      "settings.errorPrefix": "Error: ",
      "settings.signoutConfirm":
        "¿Cerrar sesión de Google? Tendrás que volver a autorizar en la próxima importación.",
      "settings.signedOut": "Sesión cerrada",
      "settings.updates": "Actualizaciones",
      "settings.checkUpdates": "Buscar actualizaciones",
      "settings.checking": "Comprobando…",
      "settings.upToDate": "Popdoc está actualizado.",
      "settings.updateDownloaded": "Actualización {v} lista — se instalará al cerrar.",
      "settings.updateError": "Error al comprobar. Inténtalo más tarde.",
      "settings.updatesHelp":
        "Popdoc busca actualizaciones discretamente al iniciar y las instala al cerrar, sin interrumpir la apertura de tus archivos.",
      "settings.version": "Versión",
      "progress.importing": "Importando…",
      "progress.opening": "Abriendo en Google Docs…",
      "progress.error": "Error",
    },
  };

  function norm(lang) {
    return STRINGS[lang] ? lang : "en";
  }

  function t(lang, key) {
    const table = STRINGS[norm(lang)];
    return (table && table[key]) || STRINGS.en[key] || key;
  }

  function applyTranslations(lang) {
    const l = norm(lang);
    document.documentElement.setAttribute("lang", l);
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      el.textContent = t(l, el.getAttribute("data-i18n"));
    });
    document.querySelectorAll("[data-i18n-ph]").forEach((el) => {
      el.setAttribute("placeholder", t(l, el.getAttribute("data-i18n-ph")));
    });
  }

  window.t = t;
  window.applyTranslations = applyTranslations;
})();
