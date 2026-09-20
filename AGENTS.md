# Regole di lavoro — rawk

## Avvio e fonti

- All'avvio leggere questo file e [STATO_PROGETTO.md](STATO_PROGETTO.md).
  Consultare poi soltanto i documenti pertinenti al task; non rileggere tutto il diario.
- Il repository Git è `rawk`; `../c_awk` è l'oracolo originale, non il repository
  su cui pubblicare modifiche. Non modificarne i sorgenti per far passare i test.
- Il piano canonico versionato è [PIANO_DI_LAVORO.md](audit-2026-09-19/PIANO_DI_LAVORO.md).
  La copia nella cartella superiore è storica: aggiornare quella in questo repository.
- Verificare branch, stato Git e file prima di assumere che il riepilogo sia attuale.

## Interazione e perimetro

- Comunicare in italiano; procedere autonomamente nell'ambito autorizzato,
  senza chiedere conferma per normali scelte tecniche, correzioni e verifiche.
- Preservare struttura, presentazione, lingua e autori della home README.
  Integrare aggiornamenti puntuali; concordare preventivamente una riscrittura
  della home o cambiamenti all'identità/presentazione del progetto.
- Un'autorizzazione a commit/push non autorizza modifiche editoriali fuori tema.
  L'utente ha autorizzato in modo permanente commit e push alla chiusura di
  ogni attività su questo repository, salvo diversa istruzione esplicita.
- Una conversazione per obiettivo coerente, mantenuta fino a verifica e consegna.
  Usare la conversazione di coordinamento per priorità e decisioni trasversali.
  Creare nuove conversazioni soltanto su richiesta dell'utente.
- Non far modificare gli stessi file da task concorrenti nella stessa checkout.
  Per lavoro parallelo autorizzato usare checkout/worktree separati.

## Implementazione e verifica

- Per bug e compatibilità: controesempio contro il C, regressione, modifica
  circoscritta, verifica. Non modificare gli expected solo per ottenere verde.
- Conservare i contratti deliberati, inclusi byte alti e NUL; distinguere profili
  verificati, differenze documentate e funzionalità ancora aperte.
- Dal repository: `make -C ../c_awk`, `cargo build --locked --release --bins`.
- Test mirati durante il lavoro; alla chiusura di modifiche al runtime:
  `CARGO_INCREMENTAL=0 bash scripts/checks.sh` e
  `CARGO_INCREMENTAL=0 cargo test --locked --release`.
- Per sole modifiche documentali verificare diff e link: non serve ripetere
  la suite runtime. Non ripetere test già verdi senza una ragione nuova.
- Il gate richiede Python 3, compilatore C, fixture originali e locale dei test.
  Linux nativo è sospeso finché non è disponibile un ambiente: `cargo check`
  per il target non dimostra linking o correttezza della libc su Linux.
- Non alterare log/evidenze storiche per ripulirli. Escludere build, cache e
  metadati AppleDouble dai commit; eliminare soltanto artefatti identificati.

## Chiusura obbligatoria del task

- Aggiornare lo stato breve e, se cambia il piano, il piano canonico.
- Compilare una consegna in `diary/` seguendo [il modello](docs/CONSEGNA_TASK.md).
- Riportare risultato, verifiche realmente eseguite, limiti, file modificati,
  stato Git e prossimo intervento. Non dichiarare completato ciò che è sospeso.
- Al termine di ogni attività, dopo le verifiche pertinenti, creare un commit
  delle modifiche del task ed eseguire il push sul repository GitHub configurato,
  senza chiedere una nuova conferma. Se non ci sono modifiche, non creare commit
  vuoti; pubblicare comunque eventuali commit locali del task ancora pendenti.
- Verificare che il commit pubblicato sia presente sul branch remoto e riportare
  hash, stato della working tree e stato CI separatamente. Se commit o push
  falliscono, riportare l'impedimento e non dichiarare il repository aggiornato.
  Non includere modifiche estranee, credenziali, build, cache o metadati AppleDouble;
  non usare force push senza un'autorizzazione specifica.
- Le evidenze del nuovo task non sostituiscono le vecchie.
