# Riconciliazione di Claude Desktop e Claude Code

18 novembre 2026 · macOS arm64 · baseline locale `25bf10a` su `main`.

## Esito

**Le 10 conversazioni principali esauriscono i transcript principali
riconosciuti nella root locale osservata.** La verifica indipendente su
disco ha trovato 11 transcript di primo livello. Uno è un fork con
riferimento al genitore nel registro Desktop. Gli altri dieci sono
conversazioni principali. Gli 8 transcript annidati sono subagent. Tutte
le 7 registrazioni locali Desktop Code si associano a un transcript
dell'archivio, compreso quello del fork. In questo perimetro non ci sono
candidati non classificati o registrazioni locali senza transcript.

La finestra aperta di Claude Desktop mostra l'intestazione **Chat e
attività** nella superficie **Chat e Cowork** e un controllo di
navigazione separato per **Code**. L'osservazione si è limitata alle
etichette di navigazione. Questo riscontro è coerente con la descrizione
dello screenshot originale, che non era allegato alla richiesta.
La sidebar e il catalogo locale rilevato in AgentMender hanno quindi
perimetri diversi. [Esperienza unificata di Chat e Cowork](https://support.claude.com/en/articles/16761823-claude-cowork-and-chat-are-one-claude) ·
[Claude Code su Desktop](https://code.claude.com/docs/en/desktop).

## Versioni e fonti

L'[audit precedente](claude-code-storage.md) ha verificato il bundle
Desktop 4.29.0 e il motore integrato 2.1.222. In questa verifica il bundle
presente in `/Applications/Claude.app` riportava **5.11492.5** e il processo
Desktop era partito dopo la data del plist del bundle. Il motore integrato
ha confermato **2.1.222** con `--version` in un processo con rete e
scrittura negate. La superficie visuale è stata quindi verificata
sull'installazione attuale. Per 4.29.0 rimane disponibile l'evidenza su
disco raccolta nel precedente audit. I transcript osservati riportano
2.1.202, 2.1.205, 2.1.209, 2.1.217, 2.1.219 e 2.1.222.

| Fonte | Evidenza verificata | Copertura di AgentMender |
|---|---|---|
| Claude Chat | «Chat e attività» è sotto «Chat e Cowork». La cronologia Chat è disponibile tramite le superfici di account | Fuori dal catalogo Code. Un eventuale provider Chat richiede una fonte autorizzata e un contratto di rimozione indipendenti. [Cronologia Chat](https://support.claude.com/en/articles/11817273-use-claude-s-chat-search-and-memory-to-build-on-previous-context) |
| Claude Cowork | La superficie può contenere attività Cowork. Cowork può usare storage locale o cloud a seconda della modalità | Fuori dal catalogo Code, inclusa la directory `local-agent-mode-sessions`. [Modalità Cowork](https://support.claude.com/en/articles/15520349-use-claude-cowork-on-web-desktop-and-mobile) |
| Code Desktop locale | Transcript con `entrypoint=claude-desktop`, più il registro `claude-code-sessions/<profilo>/<account>/local_<UUID>.json` | 10 principali e 1 fork, con 8 subagent. Le registrazioni corroborano i transcript quando identità e workspace concordano |
| Code CLI | Nessun transcript con origine `cli` o `sdk-cli` nel campione e nessun eseguibile CLI autonomo osservato nei percorsi controllati | Coperto se il transcript riconosciuto si trova in una root locale analizzata |
| Code Cloud | Una sessione può essere eseguita e conservata nell'infrastruttura cloud. Una sessione locale può anche apparire su altre superfici tramite Remote Control | La diagnostica riguarda le copie materializzate nelle root locali configurate. Il catalogo account non è stato interrogato. [Code Cloud](https://code.claude.com/docs/en/claude-code-on-the-web) · [Remote Control](https://code.claude.com/docs/en/desktop#control-which-sessions-appear-on-your-other-devices) |
| Code SSH e WSL | La documentazione distingue ambiente SSH, distribuzione WSL e ambiente locale | Fuori copertura. La scansione non apre host SSH o filesystem WSL. [Ambienti Desktop](https://code.claude.com/docs/en/desktop) |
| Archived | L'archiviazione è uno stato della sessione Code. Desktop può mostrarla in una vista dedicata | Le sessioni archiviate con transcript e stato corroborato rientrano nel totale senza filtri. [Sessioni archiviate](https://code.claude.com/docs/en/desktop#organize-and-find-sessions) |

## Riscontro locale

- Transcript in `/Users/daniel/.claude/projects/-Users-daniel`. Una root di
  configurazione leggibile, un namespace. Nessun override di root o
  namespace osservato nell'ambiente di avvio o nelle chiavi di configurazione
  locale esaminate. Non erano configurate root aggiuntive.
- Registro in `/Users/daniel/Library/Application Support/Claude/claude-code-sessions`.
  Sette registrazioni con UUID, tutte associate. Sei appartengono alle
  conversazioni principali e una al fork. Quattro conversazioni principali
  sono provate dai transcript senza una registrazione Desktop corrispondente.
  Il file condiviso dello scheduler resta fuori dalla riconciliazione.
- La root dei dati di Claude Desktop non conteneva una directory `projects`.
  Le directory di cache e i riferimenti remoti sono rimasti fuori da questo
  inventario. Nessun accesso a database dell'account o endpoint privati.
- Artefatti osservati in totale: 19 transcript, 62.945.808 byte, 7.540 righe.
  Il file più grande ha 18.659.797 byte, il file con più righe ne ha 1.952,
  il record più grande occupa 1.342.087 byte. Tutti rientrano nei limiti
  di scansione e la diagnostica non segnala limiti di elementi, byte, righe
  o tempo raggiunti. La prima richiesta API ha completato in circa 750 ms
  in questa esecuzione.
- La scansione non ha rilevato JSON malformato, entry point sconosciuti,
  conflitti irrisolti di UUID, sidechain di primo livello o trascrizioni
  parziali. Il file con UUID misti coincide con il fork corroborato.
  La ricerca, i filtri e la paginazione sono applicati dopo la classificazione.
  Sia la lista senza filtri sia la lista del singolo progetto totalizzano 10.
  Con pagine da tre, la prima e la seconda pagina contengono tre righe e
  il totale resta 10.
- Lo stato di archiviazione è attestato come non archiviato per sei
  conversazioni principali. Quattro hanno stato sconosciuto. La ricerca
  filtrata restituisce 6, 0 e 4 rispettivamente per non archiviate,
  archiviate e sconosciute. Il risultato zero per le archiviate si limita
  agli stati verificati.

## Diagnostica e verifica

**Claude Code → Diagnostica inventario Claude Code** mostra i conteggi
dello snapshot prima dei filtri, i motivi di esclusione dei transcript, le
letture parziali, l'accesso alle root e l'esito della riconciliazione Desktop.
Una registrazione conta come «senza transcript candidato» quando manca
un candidato con lo stesso UUID e l'enumerazione delle root dei progetti
è completa. Una root inaccessibile, una copertura incompleta, un UUID
mancante o un'ambiguità possono lasciare l'associazione non verificata.
Questi record restano diagnostici e non creano conversazioni.

Verifiche su macOS arm64 superate. 41 test Rust, build, fmt, Clippy con
warning negati, 5 test npm, build UI, suite HTTP e controlli architetturali.
La suite HTTP usa fixture temporanee con titoli Chat e Code identici, una
registrazione associata, una senza transcript, una senza UUID CLI, un
transcript locale senza registrazione, una sessione cloud non materializzata
in Code, una sessione archiviata, più root e una root inaccessibile.
Verifica inoltre formati e origini sconosciute, record malformati e
indipendenza dai filtri. La suite nativa Codex con l'eseguibile installato
è passata su fixture temporanee. La UI è stata verificata in Chrome a
larghezza desktop e mobile.
