---
title: "Memoria Java e Garbage Collection"
description: "Come Minecraft usa la RAM, perché allocarne di più non significa essere più veloci, cosa fanno davvero heap, pause GC e i flag di Aikar, e quando lasciare i default così come sono."
faq:
  - question: "Quanta RAM dovrei allocare a Minecraft?"
    answer: "Per Vanilla: 2-4 GB bastano e avanzano. Per una piccola istanza moddata (20-40 mod): 4 GB. Per un Modpack grande (100+ mod, stile ATM): 6-8 GB. Andare sopra gli 8 GB raramente aiuta perché il collo di bottiglia si sposta sul garbage collector, non sulla dimensione della memoria."
  - question: "Più RAM rende Minecraft più veloce?"
    answer: "Fino a un certo punto, poi no. Una volta che Minecraft ha abbastanza heap per contenere la mappa e tutte le mod caricate senza garbage collection continua, aggiungere altra memoria dà solo al GC più spazio da scansionare quando parte, il che significa pause GC più lunghe e gameplay a scatti. L'impostazione giusta è 'quanto basta', non 'quanta ne ho'."
  - question: "Cosa sono i flag di Aikar e dovrei usarli?"
    answer: "I flag di Aikar sono un set di argomenti JVM che configurano il garbage collector G1 per privilegiare pause brevi rispetto al throughput, pensati originariamente per i server Minecraft. Aiutano sui server e sui client moddati grandi. GDLauncher non li applica automaticamente; puoi incollarli in Java Arguments nei Settings dell'istanza se vuoi provarli. Non sono magia e non sono sempre più veloci."
  - question: "Perché Minecraft scatta ogni qualche secondo anche con 16 GB allocati?"
    answer: "Quasi sempre sono scatti da pausa GC, non carenza di memoria. Il garbage collector parte meno spesso se gli dai più heap, ma quando parte su un heap più grande ci mette di più. Prova ad abbassare l'allocazione (paradossalmente), o passa a un Modpack più piccolo."
  - question: "Qual è la differenza tra Xmx e Xms?"
    answer: "Xmx è la dimensione massima dell'heap (il tetto). Xms è la dimensione iniziale dell'heap (il punto di partenza). Lo slider RAM di GDLauncher imposta Xmx; Xms viene impostato a un valore sensato automaticamente. Per Minecraft, mettere Xmx uguale a Xms non aiuta in modo significativo, la JVM fa crescere l'heap quando serve dentro Xmx comunque."
---

# Memoria Java e Garbage Collection

## Come Minecraft usa la memoria

Minecraft è un programma Java. Come tutti i programmi Java, gira dentro una Java Virtual Machine (JVM) che riceve una fetta fissa della RAM di sistema. Tutto quello che Minecraft fa, chunk caricati, entità, stato delle mod, texture, vive in quella fetta.

Quando regoli **Instance Java Memory** nelle impostazioni dell'istanza di GDLauncher, stai impostando `-Xmx`, la dimensione massima dell'heap che la JVM può usare. Il codice Java stesso (allocazioni di oggetti, strutture dati delle mod, lo stato della mappa) vive in questo heap. Le texture e i buffer OpenGL vivono fuori dall'heap, nella memoria nativa, e non sono influenzati da Xmx.

## Il garbage collector è il vero collo di bottiglia

Java non libera memoria a mano; ha un **garbage collector** che periodicamente scansiona l'heap, trova oggetti che non sono più referenziati da niente, e li recupera. Minecraft moderno usa il collector **G1** di default.

Il GC gira in due modalità:

- **Young GC.** Corto, frequente. Scansiona una piccola "young generation" di oggetti creati di recente. Di solito un millisecondo o due.
- **Old GC / Mixed GC.** Più lungo, meno frequente. Scansiona il resto dell'heap. Può durare decine di millisecondi o più su un heap grosso.

Quando il GC parte, **Minecraft è in pausa**. Più lungo è l'heap, più lunghe sono quelle grandi raccolte. È per questo che aggiungere RAM oltre ciò che serve davvero al gioco peggiora gli scatti da pausa, non li migliora.

Questa è la cosa più contro-intuitiva del tuning della memoria Java: **allocare meno può essere più fluido che allocare di più**.

## La giusta quantità di RAM

Linee guida indicative, basate su quanto entra nell'heap attivo senza thrashing:

| Carico | Xmx consigliato |
|---|---|
| Minecraft Vanilla | 2-4 GB |
| Mod leggere (20-40, stile Sodium) | 4 GB |
| Modpack medio (80-120 mod) | 4-6 GB |
| Modpack grande (ATM, FTB Continents, 250+ mod) | 6-8 GB |
| Modpack "Kitchen sink" (500+ mod, pre-generazione di chunk profonda) | 8-10 GB |

Andare sopra i 10 GB raramente è utile, a meno che la documentazione del pack non lo dica esplicitamente. Alcuni pack ne richiedono di più (mod affamate di memoria come Better End o NetherEx in combinazione); segui l'impostazione consigliata dal pack.

## Come funziona lo slider di GDLauncher

Apri un'istanza, clicca la tab Settings, scorri fino a **Instance Java Memory**. Sposta il toggle per attivare un override per istanza, poi trascina lo slider; va da 1 GB fino alla RAM totale del sistema (con un avviso oltre l'80%). Il launcher converte il valore in `-Xmx<n>M` e lo passa alla JVM.

Lo stesso slider esiste a livello globale in **Settings → Java → Java Memory**, usato come default per ogni istanza che non fa override. Imposta quello globale basso per il gioco casual, alza solo per i Modpack pesanti.

## Flag di Aikar

Una lunga serie di argomenti JVM che configurano G1 per privilegiare pause brevi rispetto al throughput. Originariamente scritti per i server Minecraft ma utili anche sui client moddati. Hanno questo aspetto:

```
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ...
```

GDLauncher non li applica di default. Per usarli, incolla la stringa intera nel campo **Instance Java Arguments** nella tab Settings dell'istanza (oppure in **Settings → Java → Java Arguments** per applicarli globalmente). Gli effetti variano; il beneficio più consistente è meno pause lunghe sugli heap nel range 6-10 GB.

Qualche caveat:

- Sono pensati per versioni Java più vecchie. Su Java 17+ i default sono già piuttosto buoni e i guadagni dei flag di Aikar sono minori.
- Assumono pattern di allocazione tipo server. Su un desktop con un heap piccolo possono fare male.
- Non aggiungono memoria né cambiano quanta ne usa il gioco; cambiano solo come si comporta il collector.

Se non hai un motivo specifico per usarli, lascia il campo Java Arguments come GDLauncher l'ha impostato.

## Diagnosticare gli scatti

Se Minecraft si ferma per centinaia di millisecondi ogni qualche secondo:

1. Apri la schermata di debug F3. Guarda la riga "Mem:" (in alto a destra). Se rimbalza velocemente tra valori bassi e alti, stai vedendo churn del GC.
2. Abbassa Xmx di 1-2 GB e riprova. Contro-intuitivo, ma gli heap più piccoli fanno GC più veloce.
3. Se una mod specifica alloca come una matta (alcune mod di pre-generazione o di rendering lo fanno), lo si vede nei profiler lato mod (Spark, JmxMC). La mod potrebbe aver bisogno di un aggiornamento.
4. Se la CPU è al 100% durante le pause, il GC sta davvero lavorando duro. Abbassa Xmx ulteriormente o rimuovi mod affamate di memoria.

## TL;DR

- Imposta Java Memory a *quanto basta*, non quanta ne hai.
- Il collo di bottiglia è il tempo di pausa GC, non la dimensione grezza dell'heap.
- I flag di Aikar aiutano sugli heap grandi ma non sono una correzione magica.
- 4-6 GB sono giusti per quasi tutto ciò che è moddato.
