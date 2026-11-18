# Audit dello storage Claude Code

Osservazione del 18 novembre 2026 su macOS arm64. Baseline AgentMender
`b92223c0ec1fdf689634a72d9ff75ee2c1207965` su `main`, uguale al riferimento
locale `origin/main` e alla ref remota osservata con `git ls-remote`.
La baseline include Codex Storage Management e la verifica della rimozione
di sessioni non archiviate. I due audit Codex già presenti come file non
tracciati sono rimasti fuori dai cambiamenti.

## Installazione e fonti locali

L'audit ha letto metadati dei file e campi strutturali dei record. Le
verifiche sul binario integrato hanno eseguito soltanto `--version`,
`--help` e `project --help` con scrittura e rete negate da Seatbelt.
I test con mutazioni hanno usato esclusivamente fixture temporanee.

| Fonte | Evidenza | Ambito |
|---|---|---|
| `/Applications/Claude.app` | Bundle `com.anthropic.claudefordesktop`, versione 4.29.0 | Applicazione Desktop locale, in esecuzione durante l'audit |
| `~/Library/Application Support/Claude/claude-code/2.1.222/claude.app/Contents/MacOS/claude` | Bundle `com.anthropic.claude-code`, banner `2.1.222 (Claude Code)` | Motore Code integrato. Il PATH e i percorsi CLI convenzionali esaminati non esponevano un altro eseguibile |
| `~/.claude/projects/-Users-daniel` | 11 transcript di primo livello, 43.819.807 byte logici. Otto file `agent-*.jsonl` nelle directory `<UUID>/subagents` | Code locale. Tutti i transcript osservati riportano `claude-desktop`. Un fork contiene righe con l'UUID del genitore |
| `~/Library/Application Support/Claude/claude-code-sessions` | Sette registrazioni `<profilo UUID>/<account UUID>/local_<UUID>.json`, con `sessionId` e `cliSessionId`. Un file condiviso `scheduled-tasks.json` | Metadati locali di Code. Un record contiene `forkedFromSessionId` riferito al record Desktop del genitore |
| `~/.claude/sessions` | Directory vuota con Claude Desktop aperto | Registro locale. L'assenza di process marker lascia lo stato dei writer non verificato |
| `~/.claude/session-env`, `tasks`, `shell-snapshots`, `backups` | Nove directory session-env, tre directory tasks, shell-snapshots vuota, cinque backup di configurazione | Stato locale. La corrispondenza del nome non dimostra proprietà esclusiva o possibilità di ripristino |
| `~/.claude/history.jsonl`, `~/.claude/debug`, `~/.claude/file-history` | Assenti nel campione. Nessun sidecar dei suffissi studiati nei namespace osservati | Contratto writer, lock, sidecar e checkpoint non attribuibile a questa installazione |
| `~/.claude.json`, `~/.claude/settings.local.json` | File presenti. Una registrazione di progetto nel primo file. Nessun override pertinente nelle chiavi `env` esaminate o nell'ambiente di audit | Configurazione e possibile materiale di account condivisi. Esclusi dalla pipeline di lettura delle conversazioni |
| `~/Library/Application Support/Claude/git-worktrees.json`, `git-shadow` | Registro presente e directory git-shadow vuota. Nessun checkout attraversato | Worktree, repository, file sorgente e output dell'utente protetti |
| `~/Library/Application Support/Claude/{IndexedDB,Local Storage,Partitions,WebStorage}` | Dati Electron locali osservati per percorso | Cache e registri condivisi. La corrispondenza con le voci di Chat e attività non è verificata |
| `~/Library/Application Support/Claude/local-agent-mode-sessions` | Manifest, cache e file di lavoro di Cowork, senza transcript Code JSONL nel campione | Cowork ha un ciclo distinto. Il contenuto può includere output dell'utente ed è fuori copertura |
| Code Cloud, SSH, WSL e Claude Chat | Nessuna sessione remota importata o dedotta dai cache locali | Dati remoti o non verificabili qui. Una copia effettivamente materializzata può entrare nel catalogo solo attraverso il contratto dei transcript locali |

`CLAUDE_CONFIG_DIR` può spostare la root. Il contratto documentato di
`CLAUDE_CODE_PROJECT_DIR_NAME` può cambiare la directory direttamente sotto
`projects`. Il catalogo osserva tutte le directory riconosciute sotto
`projects` e mostra l'override del processo di avvio senza usarlo per
restringere i risultati. Il profilo di un altro processo può differire. Nel binario 2.1.222
ispezionato non è stata osservata la stringa dell'override del namespace.
L'applicabilità dell'override a quella release rimane non verificata.
[Variabili Claude Code](https://code.claude.com/docs/en/env-vars) ·
[Superfici Desktop](https://code.claude.com/docs/en/desktop) ·
[Sessioni Code Cloud](https://code.claude.com/docs/en/claude-code-on-the-web).

## Matrice comparativa

Riferimento verificato in checkout separato, Session Steward 0.12.1 al commit
[`ac80fc31f7977a00c80f5be22841a6049bb0bcf1`](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1).
Sono stati esaminati `store`, `process-state`, `events`, `tokens`, `index`,
`session-cleanup`, `session-protections`, `storage/jsonl`, `platform` e
le suite e fixture Claude con la stessa revisione. 75 test di riferimento
hanno superato l'esecuzione con Node 24.21.0. Il lavoro usa una nuova
implementazione Rust. Session Steward è MIT, copyright Mallik Cheripally,
ed è un riferimento di comportamento e formato, senza codice o dipendenza runtime importati.

| Capacità | Session Steward 0.12.1 | AgentMender e decisione |
|---|---|---|
| Discovery | `getPaths` e `platform` risolvono una home, Desktop macOS e Roaming/MSIX Windows. La fallback MSIX cerca directory con prefisso `Claude_` | Baseline rilevava una root e bundle macOS. Ora supporta default, override e root aggiuntive, candidati CLI, bundle e registrazione AppX della famiglia Claude riconosciuta. I percorsi Win32 tradizionali restano candidati |
| Inventario | `discover` aggrega copie per UUID e richiede uno stato Desktop per sessioni di quella superficie | Il catalogo Rust conserva provider, root e namespace. Le copie restano distinte, le root fisiche duplicate vengono riconosciute. Un transcript Desktop resta leggibile se il record Desktop manca |
| Titoli e attività | `readTranscriptSummary` può fermarsi appena trova i primi campi o dopo 200 righe. `readTranscriptActivity` verifica revisioni e cache e può leggere la coda. La creazione può ricadere su birthtime/mtime | La baseline leggeva 64 KiB e 96 record. Scansione limitata e cache per revisione, priorità custom → generato → prima richiesta → fallback. Creazione dalla conversazione o dal record Desktop corroborato, con mtime separato |
| Identità Desktop e relazioni | Indice per CLI UUID, evento subagent/sidechain, raggruppamento delle copie, piano directory per UUID | L'associazione Desktop richiede UUID univoco nel perimetro osservato, superficie Desktop e workspace corroborato. I fork riconosciuti e i subagent non entrano nell'inventario principale |
| Conversazione e ricerca | `events` esporta eventi tipizzati, inclusi comandi, modifiche, tool result e routing. `tokens` deduplica usage per richiesta | Preview esplicita dei soli blocchi testuali utente/assistente, omettendo controlli, output, ragionamento e indicatori di credenziali. Ricerca Unicode sui metadati, filtri, ordinamento e paginazione esistenti. La ricerca letterale globale richiederebbe un nuovo budget e un'azione esplicita. Resta fuori dal perimetro insieme alla contabilità token |
| Artefatti | `planSessionDeletion` include transcript, copie, stato Desktop, quattro suffissi JSON, registrazioni `.cast` con timestamp, directory UUID, righe history e checkpoint deep | Evidenza per il transcript selezionato, relazioni, candidati con nome esatto, artefatti condivisi protetti e righe history diagnostiche. Nome, UUID o suffisso non bastano a autorizzare una rimozione |
| Writer concorrenti | `activeSessionIds` e `claudeProcessState` considerano PID, ora di avvio e dominio. Una seconda verifica avviene dopo il backup. Il rilevamento Desktop è indicato come indisponibile | Sono distinguibili registro presente, assente e bloccato. La scansione processi da sola non impedisce riaperture prima del commit. Il caso reale dimostra che zero marker non implica zero writer |
| History | `matchingHistoryStats` seleziona per UUID. `lockHistory` usa path canonico e `proper-lockfile` 4.1.2 con stale a 10 s, heartbeat e controllo di compromissione. `rewriteJsonlFile` scrive UTF-8 con newline e poi rinomina | Conteggi limitati e diagnostici, con distinzione lettura parziale e UUID non esclusivo. Il motore 2.1.222 contiene anche un altro percorso di append. Una riscrittura deve preservare byte, separatori, righe malformate e revisione. Il lettore non tocca history, file temporanei o lock |
| Backup, rimozione e ripristino | `createBackup` e `verifyCopy` copiano e confrontano hash. Il manifest v2 include percorso e hash dei file. La cancellazione usa `fs.rm` ricorsivo e ripristino possibile in recupero o su richiesta | Requisiti futuri di ammissione nel relativo ADR. L'esecuzione ricorsiva può colpire un file aggiunto dopo il piano. Il ripristino che rimpiazza una destinazione ricreata non è ammesso. Nessuna ricevuta, backup o ripristino Claude nell'ambiente verificato |
| Policy | `session-protections` persiste regole Keep e workspace. `session-cleanup` possiede lock propri e riesamina il fingerprint | La UI Rust mantiene autorizzazione loopback, token, chiavi opache e motore Codex di ricevute. La route Claude rifiuta la mutazione prima di creare operazioni. I lock interni ad AgentMender non costituiscono un contratto con Claude |

Il probe del motore 2.1.222 espone `project purge`, a livello di progetto.
La documentazione corrente mostra anche `claude purge` e la rimozione
nativa delle sessioni Code Cloud dalla UI di Claude. Non è emerso un
comando o un'API pubblica applicabile a una sessione locale identificata
esattamente in questo archivio. Nessuno dei due scope di cleanup è
abilitato. La stessa motivazione vale se il processo Desktop viene chiuso
senza un meccanismo che impedisca una riapertura concorrente.
[Riferimento CLI](https://code.claude.com/docs/en/cli-reference).

Lettura statica del motore 2.1.222. Il percorso di fallback di history
acquisisce un lock con stale a 10 s e callback per lock compromesso. Il
modulo incluso risolve il percorso canonico, crea una directory con
suffisso `.lock` e ne rinnova il mtime a intervalli che di default valgono
metà del limite stale. La stessa funzione di history può servirsi di
un altro appender di storage. Il file di history mancava sul Mac, quindi
non è stato possibile osservare quale ramo usassero i writer reali o
verificare rinnovo, concorrenza, arresto anomalo e semantica su Windows.
La prova statica riguarda il singolo binario e non è una garanzia di
coordinamento. Il binario contiene i campi `procStart` e `procStartFt`,
ma non è stato osservato `pidDomain` né un marker vivo da associare al
registro Code di quella installazione.

## Matrice dei formati

| Versione ed evidenza | Transcript | Sidecar, registro e history | Compatibilità |
|---|---|---|---|
| Desktop 4.29.0. Motore 2.1.222. Versioni nei record 2.1.202, 2.1.205, 2.1.209, 2.1.217, 2.1.219, 2.1.222 | JSONL con UUID incorporato, `user`, `assistant`, `custom-title`, `ai-title`, subagent per UUID del genitore e ID del worker. Fork corroborato dal record Desktop | Registro processi vuoto mentre Desktop era aperto. History, file-history e suffissi studiati assenti. Registrazioni Desktop locali e scheduler condiviso | Inventario e preview macOS verificati sul Mac. Rimozione e locking non verificati |
| CLI 2.1.325 | Fixture registrata da Session Steward, `sdk-cli`. Fixture originale Rust con la stessa forma documentata, inclusi testo strutturato, duplicati e titoli tardivi | Le suite upstream esercitano sidecar, lock e history sintetici. Non equivalgono a un test del writer installato | Lettura fixture. Nessuna estensione automatica di mutazioni |
| CLI 2.1.199, 2.1.220, 2.1.228, 2.1.237, 2.1.267 e 2.1.280. Altre release Desktop elencate dal riferimento | Profili dichiarati upstream. La versione da sola non certifica uno schema | Nessuna attestazione locale per versione | Lettura se identità e schema dei record corrispondono. Cleanup disabilitato |
| Versione o installazione ignota | Richiede percorso, UUID e record compatibili. Sorgenti non classificabili, revisioni cambiate e letture tronche qualificano la copertura | Protezione predefinita | Nessuna mutazione |

## Limiti operativi

La scansione restringe la discovery a `projects/<namespace>` e ai figli
`<UUID>/subagents`. Le registrazioni Desktop sono in directory di profilo e
account UUID, su root native o esplicite. `--claude-additional-root` aggiunge
una root di configurazione. Su macOS e Linux, i file sono aperti con
descriptor-relative no-follow e controllo del volume e dei link. Su Windows
i controlli osservano gli antenati, rifiutano reparse point, verificano
volume e indice del file e leggono solo file senza hardlink. Questa
osservazione Windows non costituisce isolamento atomico rispetto a
sostituzioni concorrenti di antenati.

Il catalogo usa fino a 24 MiB e 100.000 righe per transcript, 256 MiB e
dieci secondi per ricognizione, due secondi per file e 2 MiB per record.
La cache è volatile e contiene fino a 4.096 osservazioni complete.
Quando lo scan salta un'area, titoli e date rappresentano le zone lette.
Il timestamp di creazione può essere quello della prima attività osservata
quando manca un campo esplicito. Il workspace viene omesso se incongruente
e privo di corroborazione Desktop. La preview legge finestre fino a 4 MiB e
4.096 record, restituisce al massimo 20 messaggi e 24 KiB di testo per
pagina e usa cursori server-side legati alla revisione. Il filtro di
credenziali è conservativo e l'utente apre il testo esplicitamente.

Per un futuro Standard servono identità ed esclusività di ogni singolo file,
un processo di backup verificato, spazio disponibile e coordinamento con i
writer, prima di un singolo commit autorizzato. Approfondita richiede anche
un ciclo di vita verificato per ogni checkpoint. Sul Mac erano disponibili
circa 3,6 GiB all'inizio dell'audit. È un dato istantaneo, senza valore
di ammissione per un'operazione successiva. Il contratto di recupero e le
garanzie di conservazione sono descritte nell'[ADR 0046](../adr/claude-session-cleanup.md).

## Risultati

Il catalogo reale ha trovato 10 conversazioni principali, 8 subagent e
1 fork nel singolo namespace. Tutte le 10 conversazioni principali hanno
titolo personalizzato, creazione, ultima attività e dimensione osservati.
Tutte hanno restituito testo visibile nella prima pagina della preview.
Il workspace è stato corroborato per 8. Sei record Desktop hanno attestato
lo stato non archiviato, con stato ignoto per gli altri quattro. Lo stato
di archiviazione è distinto dalla presenza di un writer.
I risultati riguardano Code locale. Chat, Cowork, dati cloud, configurazioni
e output dell'utente rimangono fuori dal contratto.

Codex è stato verificato separatamente sul Mac in sola lettura. L'API ha
mostrato 74 conversazioni principali riconosciute, 28 in due progetti
registrati (25 e 3) e 46 in Recenti. I conteggi per progetto coincidono con
le rispettive API anche usando pagine da 12. L'osservazione segnala copertura
generale parziale e lettura della membership completa. Gli screenshot
precedenti non erano allegati a questa richiesta, quindi il confronto
visivo con il loro stato di filtri o paginazione non è riproducibile.
La suite nativa usa il Codex installato 0.169.0-alpha.11.1 su copie e fixture
temporanee, compresa la rimozione di singole sessioni e sottoalberi, guardie
monouso e verifica dei record estranei.

| Verifica | Esito |
|---|---|
| macOS arm64 | Rust fmt, Clippy con warning negati, build e 40 test. Installazione npm, 5 test UI, build Vite, controlli architetturali e suite HTTP superati. Suite nativa Codex superata con il binario installato su fixture temporanee. Chrome desktop e mobile verificati con fixture e controlli di overflow e console |
| Linux arm64 | Esecuzione nel guest Linux arm64 di Docker Desktop. Installazione npm, 5 test UI, build Vite, fmt, Clippy, build Rust, 26 test Rust, controlli architetturali e suite HTTP superati |
| Windows arm64 | `cargo check --locked --workspace --all-targets --target aarch64-pc-windows-msvc` superato. Esecuzione nativa da verificare, perché non c'era un runner Windows disponibile per questo checkout locale. La matrice CI include Windows nativo |
| Session Steward | 75 test delle suite Claude e cleanup sul commit di riferimento, in checkout indipendente |
