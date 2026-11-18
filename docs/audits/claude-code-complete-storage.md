# Audit completo dello storage Claude Code

18 novembre 2026 · macOS arm64 · APFS · baseline `45e2b80` su `main`.

**Risultato.** L'inventario ha misurato **1.606.523.548 byte logici** e
**1.625.485.312 byte allocati osservati** in 7.997 file. I transcript delle
conversazioni principali, compresi i loro subagent, rappresentano
**61.020.994 byte logici**. Il catalogo ha verificato l'attribuzione esclusiva,
entro il perimetro osservato, di **59.259.727** di questi byte. Il recupero
autorizzato è **0 byte** per entrambe le modalità. Un writer Claude reale ha
continuato a scrivere sul transcript sintetico con un `flock` esclusivo
detenuto da un altro processo.

## Installazioni e root

- Claude Desktop **5.11492.5** in `/Applications/Claude.app`.
- Motore Desktop **2.1.222**, verificato con la CLI confinata, in
  `~/Library/Application Support/Claude/claude-code/2.1.222/claude.app`.
  Nessuna CLI autonoma rilevata nei percorsi già verificati. Restano 10
  conversazioni principali, 8 subagent, 1 fork e 7 registrazioni locali
  Desktop corroborate. [Riconciliazione](claude-desktop-code-reconciliation.md).
- Nessun `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_PROJECT_DIR_NAME` o
  `CLAUDE_CODE_TMPDIR` nel processo di audit. L'app supporta root aggiuntive
  esplicite. Sui sistemi Unix esamina esclusivamente il figlio
  `claude-<uid>` nelle basi temporanee, su Windows il figlio `claude`.
  In questa installazione erano assenti sia `/private/tmp/claude-501`
  sia `<TMPDIR>/claude-501`. Il motore 2.1.222 contiene il riferimento
  all'override temporaneo e alla base Unix `/tmp`. [Layout ufficiale e scratchpad](https://code.claude.com/docs/en/claude-directory#session-scratchpad-directory).

| Root | File | Byte logici | Byte allocati osservati |
|---|---:|---:|---:|
| `~/Library/Application Support/Claude` | 7.615 | 882.090.500 | 899.825.664 |
| `/Applications/Claude.app` | 308 | 655.144.988 | 656.162.816 |
| `~/.claude` | 63 | 63.487.981 | 63.680.512 |
| `~/Library/Logs/Claude` | 7 | 5.676.926 | 5.689.344 |
| `~/Library/Caches/com.anthropic.claudefordesktop` | 3 | 81.920 | 81.920 |
| `~/.claude.json` (metadati) | 1 | 41.233 | 45.056 |
| Root temporanee candidate | 0 | 0 | 0 |

L'applicazione era in esecuzione. La scansione ha rilevato i 14 collegamenti
del bundle e li ha lasciati fuori dalle somme dei file. Nessun percorso
con hardlink è emerso nel campione. La root dati Desktop risulta parziale.
I worktree e le sottodirectory prive di una relazione riconosciuta possono
restare senza enumerazione. Le misure rappresentano uno snapshot diagnostico
di archivi vivi e possono variare durante la lettura, in particolare per
cache e log. I blocchi APFS possono essere condivisi da cloni e snapshot.
La somma Unix deriva dai blocchi riportati dal filesystem e deduplica gli
hardlink osservati. Nessuna misura rappresenta lo spazio che una rimozione
libererebbe sul volume.

## Occupazione e categorie

| Categoria | Byte logici | Byte allocati osservati |
|---|---:|---:|
| Runtime Desktop e motore Code | 926.439.838 | 927.469.568 |
| Cache globali | 599.369.155 | 616.230.912 |
| Archivi di progetto Code | 63.270.531 | 63.352.832 |
| Stato Desktop condiviso | 7.322.172 | 7.655.424 |
| Log condivisi | 5.695.494 | 5.726.208 |
| Cowork, inclusi file potenzialmente utente | 4.162.644 | 4.640.768 |
| Backup protetti | 206.165 | 225.280 |
| Stato e configurazione globali | 41.397 | 53.248 |
| Stato locale Code | 11.121 | 94.208 |
| Registro Code Desktop | 4.881 | 32.768 |
| Registro di worktree | 150 | 4.096 |

Directory più pesanti per allocazione osservata. I due runtime, le cache
e lo stato Desktop rappresentano la gran parte dell'occupazione.

| Directory | File | Byte allocati osservati | Formati e protezione |
|---|---:|---:|---|
| `Claude.app/Contents` | 308 | 656.162.816 | Framework, bundle e risorse. Processo del bundle osservato |
| `Claude/Cache` | 3.862 | 510.578.688 | Cache Desktop condivisa, in prevalenza file senza estensione |
| `Claude/claude-code` | 5 | 271.306.752 | Eseguibile integrato di 271.289.792 byte logici |
| `Claude/Code Cache` | 3.302 | 88.432.640 | Cache applicativa condivisa |
| `.claude/projects` | 30 | 63.352.832 | 19 JSONL, 8 JSON e 3 TXT |
| `Claude/Shared Dictionary` | 41 | 12.132.352 | Cache applicativa condivisa |
| `Claude/local-agent-mode-sessions` | 224 | 4.640.768 | Cowork, fra cui materiale di lavoro. Contenuti esclusi dall'analisi |
| `Claude/IndexedDB` | 8 | 4.292.608 | Livello di storage condiviso, rilevati solo i metadati filesystem |

Il log `main.log` rappresentava circa 5,59 MB logici. Le cache Electron,
la cronologia Chat e le sessioni remote non sono state importate come
transcript. I file di account e configurazione sono stati misurati via
metadata. Gli output, i repository software e i worktree restano protetti.

## Attribuzione degli artefatti

- **Transcript.** `projects/<namespace>/<UUID>.jsonl` usa identità
  incorporata e revisione fisica. L'archivio contiene **62.945.808 byte**
  JSONL in tutto, inclusi i **1.924.814 byte** del fork. L'identità
  esclusiva del file viene distinta dalla presenza di riferimenti a fork,
  copie in namespace o root diverse e copertura parziale. Le varianti
  `.orphaned-…jsonl` e `.jsonl.superseded-…` sono diagnosticate per il
  nome documentato. Nessuna era presente nel campione. La stringa
  `.orphaned-` è presente nel binario 2.1.222. La variante
  `.superseded-` è documentata dalle fonti correnti, con contratto
  non verificato per 2.1.222. [Dati applicativi](https://code.claude.com/docs/en/claude-directory#application-data).
- **Subagent e tool.** I transcript dei subagent hanno locator annidato,
  identità incorporata e riferimento al genitore. Gli otto JSON adiacenti
  occupano 1.239 byte e restano candidati protetti. Tre file TXT sotto
  `tool-results` occupano **323.484 byte logici**. Una ricerca limitata
  ai 19 transcript osservati ha riscontrato ciascun basename nel suo
  transcript principale e nessuna occorrenza negli altri 18. L'API
  attribuisce la directory per posizione e mantiene proprietà non
  verificata. Il riscontro forense non prova assenza di riferimenti esterni.
  Nessun `mcp-tasks` reale nel campione. Il formato documentato include
  risultati ancora da leggere e task ripristinabili, quindi il solo stato
  completato non abilita il cleanup. [Dati applicativi](https://code.claude.com/docs/en/claude-directory#application-data).
- **Checkpoint, stato e sidecar.** `file-history`, `todos`, `debug`,
  `history.jsonl` e i sidecar JSON o CAST ricercati erano assenti.
  `session-env` contiene nove sottodirectory vuote. `tasks` contiene
  tre directory e 26 file. Le corrispondenze per UUID sono candidati,
  mentre il ciclo di vita del task e il valore di rewind non sono attestati.
  `shell-snapshots` e `sessions` erano vuote. I cinque backup locali
  sono protetti. Anche `plans`, `usage-data`, `stats-cache.json`,
  `feedback-bundles`, plugin e skill erano assenti dalla root Code osservata.
- **Immagini e temporanei.** Nessuna directory `image-cache`,
  `uploads`, `dev-mods` o `paste-cache` nella root osservata.
  `image-cache` e `paste-cache` sono riconoscibili nel binario
  2.1.222. Il layout documentato sposta le immagini nel temp dopo
  2.1.274. La UI mostra entrambe le posizioni come candidati quando
  presenti e preserva lo scratchpad come possibile output utente.
  Nessun scratchpad, immagine o upload è stato attribuito alle sessioni
  reali in questa scansione. Nessuna ricerca estesa al resto della home o
  del temporaneo di sistema. [Immagini e temporanei](https://code.claude.com/docs/en/claude-directory#application-data).
- **Registrazioni Desktop.** Sette record individuali sono associati
  mediante identificatori e workspace, con riferimenti a fork preservati.
  Il registro da 4.881 byte comprende anche il file condiviso dello
  scheduler. L'associazione di un record a un transcript non certifica
  il contratto di modifica del registro, dei servizi di sincronizzazione
  o dello scheduler.

## Anteprime di tre sessioni reali

Identificatori abbreviati. Le misure dei piani non includono gli artefatti
da preservare. I valori rappresentano i candidati osservati e l'esclusività
dei transcript nel catalogo locale.

| Sessione | Rollout | Subagent, sidecar e altri dati locali | Checkpoint e risorse esterne |
|---|---:|---|---|
| `0a178201` | 6.612.334 B | Quattro subagent, 18.352.598 B. Quattro JSON adiacenti, 644 B, protetti. Registrazione riconciliata, 625 B. Stato session-env vuoto | Nessun checkpoint, temp, upload o sidecar di primo livello osservato |
| `47cf1844` | 1.609.239 B | Due tool-result, 260.426 B. Stato task, otto file per 3.847 B. Registrazione riconciliata, 658 B. Stato session-env vuoto | Nessun checkpoint, temp, upload o sidecar di primo livello osservato |
| `1d1b9521` | 18.659.797 B | Stato session-env vuoto. Nessuna registrazione Desktop, subagent o sidecar osservati | Nessun checkpoint, temp, upload o risultato tool osservato |

| Sessione | Standard, file / byte logici | Approfondita, file / byte logici | Byte esclusivi verificati | Recupero autorizzato |
|---|---|---|---:|---:|
| `0a178201` | 6 / 24.965.557 | 6 / 24.965.557 | 24.964.932 | 0 |
| `47cf1844` | 10 / 1.613.744 | 12 / 1.874.170 | 1.609.239 | 0 |
| `1d1b9521` | 1 / 18.659.797 | 1 / 18.659.797 | 18.659.797 | 0 |

Per le tre sessioni, history non osservata, registro dei processi locale
vuoto, processo Claude Desktop osservato, legacy sidecar di primo livello
assenti, stato backup **non creato**. Al momento della misura il volume
riportava **2.313.555.968 byte disponibili**. Lo spazio disponibile è
diagnostico e andrebbe ricontrollato insieme alla destinazione di recupero
prima di qualsiasi modifica.

## Rimozione e writer

La CLI installata espone `project purge` a livello di progetto.
La documentazione corrente espone l'eliminazione delle sessioni cloud
nella loro interfaccia e non fornisce un'operazione per-sessione locale
applicabile all'identità verificata qui. `purge` e `rm` restano fuori
dal percorso di rimozione di una conversazione. [CLI](https://code.claude.com/docs/en/cli-reference) ·
[Sessioni](https://code.claude.com/docs/en/sessions) ·
[Desktop](https://code.claude.com/docs/en/desktop).

La prova [native_claude_writer_probe.py](../../scripts/native_claude_writer_probe.py)
usa il motore reale 2.1.222 con home e archivio sintetici. Seatbelt consente
scritture soltanto nel fixture e rete in uscita soltanto verso il mock
loopback. Il processo di verifica detiene un `flock` esclusivo sul
transcript. Un secondo processo Claude esegue `--resume` e aggiunge dati
allo stesso file, da **1.257 a 3.440 byte**, mentre il lock è detenuto.
L'hash della lettura iniziale diventa quindi obsoleto anche con il lock.
La prova copre il writer CLI del motore installato. L'assenza di marker
e una chiusura momentanea di Desktop lasciano aperta la possibilità di
un nuovo writer, inclusi SDK e processi di background. La prova non
ha creato `history.jsonl`. Il precedente audit ha già individuato due
percorsi di append, con lock a directory a 10 secondi per il fallback.
Rimangono da verificare l'esclusione di tutti gli altri appender e
la semantica di sostituzione del registro. [Audit del writer history](claude-code-storage.md#matrice-comparativa).

| Capacità | Stato | Evidenza necessaria per abilitarla |
|---|---|---|
| Standard | Sola anteprima, 0 byte autorizzati | Operazione nativa per identità locale o barriera verificata contro restart e riaperture da Desktop, CLI, SDK e writer background. Va mantenuta tra revisione, backup e commit |
| Approfondita | Sola anteprima, 0 byte autorizzati | Contratto Standard più prova separata di proprietà, riferimenti, stato e ciclo di vita di ciascun checkpoint, task, file temporaneo e allegato |
| History | Conteggio diagnostico | Lock e revisione coordinati con tutti gli appender, preservazione byte-per-byte dei record estranei e sostituzione verificata |
| Backup e conferma | Disponibilità del volume osservata su Unix | Perimetro immutabile e isolato, spazio ricontrollato, manifest con hash e revisioni e conferma monouso |
| Recovery | Nessun backup creato | Contratto di backup verificato e prove di guasto e ripristino con verifica della destinazione prima di ogni scrittura |

Session Steward al commit `ac80fc3` rende riproducibili pianificazione,
protezione per scope, confronto delle copie, rewrite sotto lock e fixture
di ripristino. La seconda lettura del registro processi e il lock
dell'app non impediscono a un writer esterno di riaprire il transcript.
La rimozione ricorsiva della directory UUID e il ripristino che può
sostituire una destinazione non entrano nel contratto di AgentMender.
Le **75 prove** delle suite Claude e cleanup di Session Steward sono
passate su fixture separate. [Implementazione di riferimento](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1) ·
[ADR 0046](../adr/claude-session-cleanup.md).

## Budget e copertura

L'inventario usa 60.000 voci, profondità 32 e un budget di sei secondi
per la passeggiata dei metadati. Per sessione, il budget dei metadati è
8.192 voci e tre secondi, con un limite di 128 artefatti restituiti.
Il lettore della cronologia usa il budget separato del preflight.
L'API usa il worker di lettura e una cache breve. Su Unix la passeggiata
usa descrittori e stat no-follow, con rifiuto di volumi inattesi. Su
Windows usa controllo dei reparse point e identità delle directory
osservabili. L'allocazione e il conteggio degli hardlink non sono
misurati in Windows. I conteggi globali dei transcript provengono
dallo snapshot del catalogo, le anteprime riverificano le revisioni
prima e dopo la misura. Gli archivi reali sono stati letti in sola lettura.

## Verifica della consegna

Le fixture isolano gli archivi dai dati reali. La suite HTTP verifica
protezione di hardlink e symlink, rifiuto di una registrazione cambiata e
di un transcript sostituito a parità di contenuto e mtime, rifiuto del
cleanup e conservazione byte-per-byte di transcript, history e artefatti
estranei. Le prove di guasto durante backup, cancellazione e ripristino,
incluso lo spazio insufficiente, dipendono da percorsi di scrittura Claude
che restano disabilitati. La prova nativa riproduce il writer concorrente
sul transcript con il lock detenuto.

| Ambiente | Esito |
|---|---|
| macOS arm64 | Format, Clippy, 41 test Rust, build, 5 test npm, build UI, architettura e HTTP. Prova writer Claude 2.1.222 e regressione nativa Codex 0.169.0-alpha.11.1, incluse rimozione di sessioni e progetti nelle fixture. Viste Storage verificate a 1.440 e 390 px |
| Linux arm64, container Debian | Format, Clippy, 27 test Rust, build, 5 test npm, build UI, architettura e HTTP |
| Windows arm64 | `cargo check --workspace --all-targets` per `aarch64-pc-windows-msvc`. L'esecuzione su un host Windows resta da verificare |
