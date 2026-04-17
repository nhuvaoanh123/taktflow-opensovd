# Academic and conference references

Fetched 2026-04-16. Abstracts are free; full papers are paywalled behind
SAE Mobilus, IEEE Xplore, or ResearchGate.

---

## SAE 2024-01-7036 — "Diagnostics of Automotive Service-Oriented Architectures with SOVD"

<https://saemobilus.sae.org/papers/diagnostics-automotive-service-oriented-architectures-with-sovd-2024-01-7036>

**Authors**
- Boris Boehlen — DSA Daten- und Systemtechnik GmbH.
- Diana Fischer — DSA Daten- und Systemtechnik GmbH.
- Jue Wang — DSA Electronic Technology Co. Ltd.

**Published**: 2024-12-13.

**Presented at**: SAE 2024 Intelligent and Connected Vehicles Symposium,
Shanghai, 2024-09-22.

**Abstract (verbatim snippets)**:
> "The term Software-Defined Vehicle (SDV) describes the vision of
> software-driven automotive development, where new features, such as
> improved autonomous driving, are added through software updates."
>
> "ASAM's SOVD API (ISO 17978) fills this gap by providing a foundation
> for diagnosing vehicles with service-oriented architectures."
>
> "Monitored values must be aggregated and correlated to error events
> before cloud transmission."

**Keywords / topics**: Intelligent transportation systems, Communication
protocols, Diagnostics, On-board diagnostics (OBD), Connectivity,
Computer software and hardware, Cloud computing, Autonomous vehicles,
Architecture, Data exchange.

**Relevance to Taktflow**: DSA explicitly uses SOVD as the foundation for
diagnosing HPC-based SDVs. Matches Taktflow's target architecture.

---

## SAE 2025-01-8081 — "Options for Introducing SOVD in SDV Architectures"

<https://saemobilus.sae.org/papers/options-introducing-sovd-sdv-architectures-2025-01-8081>

**Authors**
- Julian Mayer — Softing Automotive Electronics GmbH.
- Stefan Bschor — Softing Automotive Electronics GmbH.
- Oliver Fieth — Softing Automotive Electronics GmbH.

**Abstract (verbatim)**:
> "The trend for the future mobility concepts in the automotive industry
> is clearly moving towards autonomous driving and IoT applications in
> general. Today, the first vehicle manufacturers offer semi-autonomous
> driving up to SAE level 4. The technical capabilities and the legal
> requirements are under development. The introduction of data- and
> computation-intensive functions is changing vehicle architectures
> towards zonal architectures based on high-performance computers (HPC).
> Availability of data-connection to the backend and the above explained
> topics have a major impact on how to test and update such
> 'software-defined' vehicles and entire fleets. Vehicle diagnostics will
> become a key element for onboard test and update operations running on
> HPCs, as well as for providing vehicle data to the offboard backend
> infrastructure via Wi-Fi and 5G at the right time. The standard for
> Service Oriented Vehicle Diagnostics (SOVD) supports this development.
> It describes a programming interface for a diagnostic system integrated
> in the vehicle. It's an API for all vehicle architectures capable of
> fully onboard testing and updating software. Nevertheless, many car
> manufacturers will have to define a migration path for introducing it
> step-by-step throughout the entire life cycle. For this reason, a
> combination of SOVD-capable and UDS-based legacy onboard and offboard
> systems will become essential."

**Keywords / topics**: Autonomous vehicles, Diagnostics, Embedded
software, Computer software and hardware, Manufacturing systems,
Architecture.

**Relevance to Taktflow**: The paper addresses exactly the hybrid
UDS+SOVD architecture Taktflow is building (CDA on one side, SOVD-native
HPC apps on the other). The migration-path framing is directly relevant
to our deployment plan.

---

## Related conference papers (URLs captured, abstracts not all extracted)

| Title | Venue / year | URL | Notes |
|---|---|---|---|
| "An Architecture for Vehicle Diagnostics in Software-Defined Vehicles" | ResearchGate 393321067 | <https://www.researchgate.net/publication/393321067_An_Architecture_for_Vehicle_Diagnostics_in_Software-Defined_Vehicles> | ResearchGate 403'd our WebFetch; abstract visible in a real browser. |
| "Diagnostics of Automotive Service-Oriented Architectures with SOVD" (ResearchGate re-host) | ResearchGate 387023102 | <https://www.researchgate.net/publication/387023102_Diagnostics_of_Automotive_Service-Oriented_Architectures_with_SOVD> | Duplicate of SAE 2024-01-7036 above. |
| "Vehicle Software Configuration Strategies for SDVs: A Comparative Analysis" | DSM 2025 | <https://www.designsociety.org/download-publication/48685/vehicle_software_configuration_strategies_for_sdvs_a_comparative_analysis_of_on-board_and_off-board_methods> | Not SOVD-specific; background on SDV configuration. |
| "Rethinking Vehicle Architecture Through Softwarization" | IEEE | <https://ieeexplore.ieee.org/iel8/6287639/6514899/11078249.pdf> | Paywalled; IEEE access required. |

---

## Podcast

**Sibros "Point B" E22** — "SOVD, Autonomy, and Automotive Standardization with ASAM"
- Date: 2024-07-30.
- Duration: 38:19.
- Host: Steve Schwinke.
- Guests: **Ben Engel** (CTO, ASAM) and **Ahmed Sadek** (Product &
  Technology Manager, ASAM).
- Page: <https://www.sibros.tech/point-b/e22> (metadata only; audio via
  Spotify/Apple Podcasts/YouTube).

Useful attribution data for "who's in charge of SOVD at ASAM" even though
the transcript itself is not on the page.
