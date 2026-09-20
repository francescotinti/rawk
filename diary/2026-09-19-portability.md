# Portabilità — 19 settembre 2026

Stato: infrastruttura Linux predisposta, controlli Rust per i target superati;
esecuzione nativa Linux ancora da verificare. Non è una certificazione Linux.

## Intervento

Il workflow mantiene il gate completo Darwin e aggiunge due job nativi Ubuntu
24.04, x86-64 e ARM64, indipendenti in caso di errore. I job compilano lo stesso
oracolo C fissato dalla Fase 7 e usano Rust 1.96.0; generano esplicitamente le
locale en_US.UTF-8 e tr_TR.UTF-8. Log e inventario dei driver sono conservati
come artefatti distinti per piattaforma. Nessun push né esecuzione CI remota.

`scripts/check_portability.sh` esegue fmt, Clippy, build release, test unitari
e undici suite di integrazione in debug e release: Unicode, formattazione
dinamica, regressioni dei driver, input, runtime, dati, linguaggio, array a byte,
printf a byte, regex a byte e harness. Include i 300 casi originali UTF-8.

I contratti di corpus e diagnostiche registrati su Darwin restano nel gate
completo `scripts/checks.sh`: non vengono rinominati o accettati automaticamente
come baseline Linux. L'inventario completo dei driver in CI Linux è diagnostico;
le suite del gate sono vincolanti e un fallimento fa fallire il job.

Un nuovo test verifica separatamente entrambi gli interpreti: emoji di lunghezza
1 in UTF-8 e 4 in C; conversione di é in É nelle due locale UTF-8. Impedisce che un confronto
differenziale passi perché entrambi gli interpreti sono ricaduti sulla locale C.
Le locale mancanti producono un fallimento esplicito, non uno skip.
La prima esecuzione ha evidenziato un’aspettativa non portabile nel nuovo test:
la libc Darwin converte i in I anche nella locale turca. Il test richiede ora
una conversione comune alle piattaforme; i casi turchi restano confrontati
con l’oracolo locale, senza imporre le tabelle di una libc all’altra.

## Riproduzione

Dal repository rawk, con ../c_awk compilato e le due locale UTF-8 disponibili:

```sh
bash scripts/check_portability.sh
```

Controlli senza esecuzione, effettuabili anche da macOS con i target rustup:

```sh
cargo check --locked --all-targets --target x86_64-unknown-linux-gnu
cargo check --locked --all-targets --target aarch64-unknown-linux-gnu
cargo check --locked --all-targets --target x86_64-apple-darwin
```

Tutti e tre superati. `cargo check` verifica il codice e i tipi per il target,
ma non esegue il linker né dimostra la presenza dei simboli libc o la correttezza
delle locale sul sistema destinazione. Queste verifiche spettano ai job nativi.
Workflow YAML e sintassi Bash verificati localmente.

## Limiti e attività successive

La macchina disponibile è Darwin ARM64, senza runtime Linux/container installato.
L'esecuzione nativa Linux resta pendente: occorre eseguire la matrice CI, leggere
gli artefatti e risolvere eventuali divergenze prima di dichiarare supporto Linux.
Windows, musl e codifiche legacy non UTF-8 non sono coperti da questa tranche.
Il runtime impiega API Unix e locale POSIX; non è stata aggiunta una promessa di
portabilità Windows. Il runtime AWK non è stato modificato in questo intervento.

## Risultati locali finali

- Gate di portabilità: exit 0, 63 test debug e 63 release, inclusi 300 casi
  originali UTF-8 eseguiti dai driver T.utf/T.utfre per ciascun profilo di build.
- Gate completo Darwin: exit 0, 125 test debug; build release, fmt, Clippy,
  igiene del repository e confronto XML superati.
- Check di tutti i target: Linux glibc x86-64, Linux glibc ARM64 e Darwin x86-64,
  exit 0. Nessun binario Linux eseguito.
- Sintassi Bash e parsing YAML del workflow superati.

Log e impronte dei file di questa tranche in
[verification-portability-2026-09-19](verification-portability-2026-09-19/).
