# Diagnosi del percorso host Luna per la ricerca sulle carte

12 novembre 2026. Baseline verificata `main` a
`687191141a1a0fb69818459dd96868a6b3b20f70`.

## Causa verificata

I due tentativi «Trova sessioni sulle carte» delle 14:02 e 14:06, ora di
Roma, erano chat Codex Desktop con directory di lavoro sotto
`Documents/Codex`. I rollout registrano quel workspace, il turn context
`gpt-6.1-terra` con effort `high`, tre chiamate di introspezione dei
tool e il completamento della risposta del modello originario. Il log
App Server non registra tentativi di spawn o rifiuti del modello in
quei turni.

| Verifica | Riscontro |
| --- | --- |
| Istruzioni di progetto | I turni erano fuori dal checkout AgentMender e i messaggi di istruzione registrati non contenevano la sua guida Two-Stage. |
| Ruoli nominati | `session_planner` e `session_evaluator` erano configurati nel checkout. Le chat erano fuori da quello scope e non c'erano copie dei ruoli nel registro personale. |
| Agente non avviato | Il Planner non è partito. L'Evaluator non è stato raggiunto e il resolver non ha ricevuto `start`. |
| Tentativo nominato | Le uniche azioni erano lookup del catalogo. Zero richieste di spawn nominate. |
| Modello ed effort risolti | Gli eventi riportano Terra High per il turno originario. Nessun modello o effort è stato risolto per un figlio. |
| Fallback esplicito | Zero richieste esplicite a GPT-6 Luna Medium. |
| Errore concreto | L'interruzione storica è avvenuta nell'orchestrazione. Nessun rifiuto di disponibilità, configurazione o permessi è stato osservato dopo una richiesta Luna. |
| Eseguibile e catalogo | I processi Desktop caricavano il binario Two-Stage ricompilato. Il catalogo di quel binario esponeva `publish` e le cinque fasi. Il suo handshake pubblicava ancora la guida al vecchio ruolo `session_relevance` e a un solo delegato. |

La guida obsoleta proveniva da `ServerHandler::get_info` ed era visibile
nelle descrizioni MCP catturate dai due rollout. L'ultima chat ha cercato
`session_relevance` tra i tool MCP, che elencano strumenti e non i ruoli
del client. Ha quindi concluso che Luna fosse indisponibile prima di
tentare l'esecuzione. Le configurazioni nominali sono legate alla
directory del progetto. [Ruoli Codex](https://learn.chatgpt.com/docs/agent-configuration/subagents) · [Scope di AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)

Il controlled App Server legge i file di istruzione direttamente dal
checkout e chiama `thread/start` con modello, effort e rifiuto del
fallback del provider. Può verificare gli eventi sulla sua stessa
connessione e attendere l'uscita del processo del ruolo.
[Eventi App Server](https://learn.chatgpt.com/docs/app-server)

## Correzione

Il nuovo handshake allinea i nomi dei due ruoli alle fasi del catalogo
MCP e indica il fallback esplicito a GPT-6 Luna Medium. Quando il
client Desktop non espone o non permette di verificare l'avvio e la
chiusura dei ruoli, la guida indica il driver App Server esistente per
l'intero flusso. Il driver registra il ruolo, la fase e l'errore
osservato. Un contesto rifiutato con `invalid_scope` può essere
riformulato dallo stesso Evaluator sulla stessa revisione del run.

Il motore, lo snapshot UUID, le regole di classificazione e la sequenza
Two-Stage mantengono il contratto esistente. Il confronto di handshake
sui due eseguibili ha rilevato `publish` in entrambi, il ruolo legacy solo
nell'installazione precedente e il fallback controllato nel binario
corretto. La verifica end-to-end del client Desktop ordinario resta
aperta. L'accettazione usa il vero Codex App Server locale.

## Prova reale sulle carte

Il driver ha ripreso la richiesta originale sul corpus Codex locale
tramite il percorso App Server esplicito. Un primo tentativo ha
completato l'avvio di entrambi i ruoli e la scansione, poi App Server ha
restituito `usageLimitExceeded` durante un turno dell'Evaluator. Era un
limite osservato dell'esecuzione reale, distinto dal caso Desktop. Dopo
il ripristino della quota, un secondo tentativo ha rilevato un
`invalid_scope` in una richiesta di contesto. Il driver corretto consente
di riformulare questo rifiuto nello stesso thread.

Il run finale ha registrato due identità `thread/started` distinte,
entrambe con modello risolto `gpt-6-luna` e effort `medium`. Il Planner ha
completato un turno ed è uscito prima del retrieval. Rust ha esaminato
541 fonti su 541 e ammesso 72 candidati. Lo stesso Evaluator ha completato
28 turni e consegnato 15 batch validi per tutti e 72 i candidati. Quattro
payload rifiutati sono stati corretti nello stesso thread. L'Evaluator è
uscito prima della pubblicazione. Il run è terminato con
`retrieval_complete:true`, `evaluation_complete:true` e zero candidati
pendenti.

| Esito | Numero |
| --- | ---: |
| Sessioni confermate e pubblicate | 15 |
| Candidati non confermati | 55 |
| Alternative plausibili pubblicate nella seconda tabella | 12 |
| Candidati respinti | 2 |

Entrambe le tabelle sono state verificate per cardinalità, UUID univoci
e assenza del chiamante. Le fonti avevano cronologia parziale e 469
sono rimaste irrisolte rispetto al filtro lessicale. La scansione
completa della coorte vale per la cronologia leggibile e la valutazione
completa vale per i 72 candidati ammessi. Il run ha richiesto 74
chiamate MCP e 403,02 s. Planner, retrieval, Evaluator e pubblicazione
hanno richiesto rispettivamente 4,49 s, 57,35 s, 341,10 s e 87 ms. Gli
eventi host riportano 15.352 token per il Planner e 1.316.142 per
l'Evaluator. Traccia privata e tabelle con i titoli effettivi sono
conservate fuori da Git.

## Verifiche

La build locked, Clippy con warning negati, il formato e i controlli
di architettura, lifecycle e sicurezza sono passati su macOS. La suite
workspace ha passato 581 test con 8 opt-in ignorati. Sono passati 14
test del driver e 6 test del validatore semantico. Il test di
regressione dell'handshake verifica entrambi i ruoli, `publish:true` e
il fallback App Server. Una prova con eseguibile Codex mancante ha
registrato `ENOENT` all'avvio del Planner prima della scansione.

Implementazione locale in `fix/luna-host-routing` al commit `99f71df`.
