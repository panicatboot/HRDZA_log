# ROADMAP

## Architecture générale

TUI (Terminal UI)

    - Utiliser ratatui (fork de tui-rs) ou cursive pour gérer fenêtres, menus, formulaires de filtres, tableaux.
    - Séparer UI ↔ logique grâce à un pattern MVC ou actor-based (ex. avec tokio::sync::mpsc).

Sources de logs

    - Journal systemd : via systemd-journal pour lire en continu (journalctl —follow) et indexer.
    - Fichiers texte : /var/log/auth.log, /var/log/openvpn.log… utilise un watcher basé sur notify pour capter les ajouts.
    - Audit Linux : pour actions sensibles, via libaudit (auditd) et le crate audit.
            Exécution de binaires sensibles (execve sur /usr/bin/sudo, /bin/passwd, etc.)
            Modifications de fichiers système (création/suppression/rename de /etc/shadow, /etc/sudoers)
            Changements de droits ou d’UID/GID (via setuid, setgid)
            Succès/échec de connexions PAM (login SSH, su, sudo)
            Appels système réseau (ouverture de socket sur ports réservés <1024)
            Montage/umount de volumes

## Filtrage et recherche

Filtres dynamiques

    - Par date/heure (plage ISO 8601), utilisateur, process, niveau (INFO/WARN/ERR), IP source…
    - Affiche un formulaire “Apply filter”, rebuild view et trie en O(n) ou indexe préalablement (map time→lines).

Fuzzy search

    - Intègre un “instant search” à la façon fzf avec le crate skim.

## Graphiques dans le terminal

Courbes et barplots

    - Crate tui + son module widgets::Chart pour dessiner des graphiques ASCII/Unicode.
    - Alternativement, termplot ou crate plotlib.

Heatmap d’activité dans le TUI

    - Aggréger tes logs en matrice 7 jours × 24 heures (ou 7×12 demi-heures).
    - Afficher un Table de ratatui où chaque cellule est coloriée en nuance de gris ou de couleur selon la valeur (via Cell::style(Style::default().bg(color))).
    - Automatiser le calcul d’intensité : normaliser les counts en 0–5 niveaux et choisir un bloc Unicode (▁▂▃▄▅▆▇).
    Résultat : un carré 7×24 qui ressemble à la heatmap GitHub

Exemple : tracer le nombre de connexions SSH par jour entre 1jour et 7 jours :

    Aggrégation des timestamps en buckets journaliers.
    Génération d’un Dataset::default().name("SSH").data(&[(0, count0), …,(6, count6)]).
    Insertion dans le widget Chart.

## Export en PDF

Génération PDF

    - Utiliser genpdf, qui propose un mécanisme de PageDecorator permettant d’insérer un header/ footer sur chaque page. Créer un décorateur qui affiche le logo ou le nom « hrdza_log.rs » en haut.
    - Partir d’un template HTML/CSS (via un fichier externe) et passer par wkhtmltopdf (invoqué depuis Rust). L’intérêt : définir un header project-wide dans ton <header> HTML, plus facile à styliser.

## Interaction avec les processus

Récupération des PID

    - Crate sysinfo ou psutil pour lister processus, récupérer PID, état, uptime.

Action “kill”

    - Via crate nix: nix::sys::signal::kill(Pid::from_raw(pid), Signal::SIGTERM).

Suivi du temps de connexion

    - Pour SSH/OpenVPN, croise /var/run/utmp (crate utmp_rs) pour connaître heure de login/logout.
    - Calcule durée = logout – login.

## Surveillance de périphériques (caméra, VPN…)

Caméra

    - Surveille l’ouverture de /dev/video* : via inotify (crate notify) pour détecter “OPEN” sur l’appareil.
    - Logge “caméra allumée” en tant qu’événement horodaté.

OpenVPN

    - Parse /etc/openvpn/*.log ou interroge management interface TCP (configurable) pour lister clients connectés.

Imprimante Wi-Fi

    - Interroge CUPS via son API (utiliser le crate cups) pour lister les imprimantes actives et leur état.

Microphone

    - Avec ALSA (alsa-sys) ou PulseAudio (libpulse-binding), vérifier si un stream est ouvert (callback sur stream_state).

USB / USB-C / HDMI

    - Employer libudev (crate udev) pour détecter en temps réel les ajouts/retraits de dispositifs et récupérer leurs noms (ID_MODEL, ID_VENDOR).

    - Pour le HDMI, surveiller le sous-arbre DRM (/sys/class/drm/) ou interroger xrandr en shell via std::process::Command.

## Suivi d’activité des applications (nouveau)

    - Détection de l’application active
    - Sur Kubuntu (X11) : interroger _NET_ACTIVE_WINDOW puis _NET_WM_PID via le crate x11rb.
    - Journalisation des sessions
    - Polling toutes les 5 s : récupérer PID, nom de l’appli, titre de fenêtre et timestamp.
    - Stocker dans SQLite (rusqlite) : table (ts_start, ts_end, pid, app_name, window_title).
    - Affichage TUI
    - Menu « Usage apps » : tableau résumé avec durée totale par application, pourcentage du temps actif, filtres jour/semaine.
    - Graphique barplot ou heatmap d’activité (heures vs. minutes) pour chaque appli.
    - Export
    - Chapitre « Applications » dans le rapport PDF, mêmes décorateurs et template.
    - Option : lire l’API d’ActivityWatch (Open Source) au lieu de réinventer.


# Approche incrémentale : commencer par le TUI + logs SSH, ajouter audit, puis heatmap, puis PDF, enfin périphériques.
# Modularité : exposer chaque source (journal, audit, udev) comme un module/plugins pour évoluer facilement.

## Priorisation plus fine

    Phase 1 : TUI minimal + ingestion des logs texte (auth.log, journalctl).
    Phase 2 : filtres et recherche (date, user, process), avec unit tests.
    Phase 3 : intégration Auditd + module libaudit.
    Phase 4 : heatmap d’activité dans le TUI.
    Phase 5 : export PDF (d’abord genpdf, puis template HTML si besoin).
    Phase 6 : modules périphériques (udev, CUPS, PulseAudio…).


Configuration et persistance

    Fichier de config (TOML/YAML) pour paramétrer les sources, les niveaux d’alerte et le style du TUI.
    Stockage local (par exemple SQLite via rusqlite) pour indexer et historiser les logs, accélérer les recherches répétées.

Tests et qualité

    Ajouter des unit tests pour chaque module (parsing, agrégation, PDF).
    Intègrer du linting (Clippy) et du formatting automatique (rustfmt) dans un workflow CI (GitHub Actions, GitLab CI…).


Observabilité

    Instrumenter l'application avec des métriques (via prometheus exporter) pour surveiller la charge CPU/mémoire, le nombre d’événements traités.
    Prévoir un niveau de log (DEBUG/INFO/WARN/ERROR) pour le moteur lui-même.

Documentation utilisateur

    Un README clair, un guide de démarrage rapide.






