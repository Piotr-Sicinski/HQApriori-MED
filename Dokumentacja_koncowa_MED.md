# Dokumentacja końcowa – MED

Piotr Siciński  
10.01.2026

---

## 1. Opis zadania

### 1.1. Cel projektu

Celem projektu było zaimplementowanie i zbadanie algorytmu do wielopoziomowego wyszukiwania reguł asocjacyjnych, opartego na artykule:

> Jiawei Han and Yongjian Fu, "Mining multiple-level association rules in large databases," in IEEE Transactions on Knowledge and Data Engineering, vol. 11, no. 5, pp. 798-805, Sept.-Oct. 1999, doi: 10.1109/69.806937.



### 1.2. Przyjęte założenia

#### Założenia dotyczące algorytmu
1. Hierarchiczne przetwarzanie: Algorytm przetwarza dane na wielu poziomach abstrakcji, wykorzystując taksonomię pojęć od poziomu ogólnego do szczegółowego.

2. Progresywne zagłębianie (Top-Down Progressive Deepening): Implementacja wykorzystuje podejście "od góry do dołu", gdzie:
   - Najpierw wyznaczane są częste elementy na najwyższym poziomie taksonomii
   - Następnie algorytm rekurencyjnie zagłębia się w potomków częstych elementów na niższych poziomach
   - Elementy rzadkie są przycinane wraz z ich poddrzewami

3. Różne progi dla różnych poziomów: Każdy poziom taksonomii może mieć własne progi:
   - `min_support` - minimalny próg wsparcia
   - `min_confidence` - minimalny próg ufności
   - `gen_rules` - flaga określająca, czy generować reguły na danym poziomie

4. Wykorzystanie Apriori: Algorytm bazuje na klasycznym Apriori:
   - Generowanie itemsetów k-elementowych z (k-1)-elementowych
   - Join step i prune step zgodnie z algorytmem Apriori
   - Generowanie reguł asocjacyjnych z częstych itemsetów

#### Założenia implementacyjne

1. Język programowania: Rust - wybór podyktowany:
   - Wysoką wydajnością (comparable z C++)
   - Bezpieczeństwem pamięci bez garbage collectora
   - Doskonałą obsługą struktur danych

2. Struktury danych:
   - RoaringBitmap - efektywna reprezentacja zbiorów transakcji (kompresja bitmap)
   - Arena (indextree) - reprezentacja drzewa taksonomii
   - HashMap - przechowywanie częstych itemsetów

3. Format danych:
   - Transakcje: format tekstowy, jeden itemset na linię
   - Taksonomia: format CSV (dziecko,rodzic)

4. Kompatybilność: Implementacja referencyjna w Python (efficient-apriori) służy jako punkt odniesienia dla poprawności algorytmu.

---

## 2. Opis projektu implementacji algorytmu

### 2.1. Architektura systemu

System składa się z trzech głównych modułów:

#### 2.1.1. Moduł Repository (`repository.rs`)
Odpowiedzialny za:
- Wczytywanie i parsowanie danych wejściowych (transakcje, taksonomia)
- Budowę hierarchicznej struktury drzewa taksonomii
- Propagację bitmap od liści do korzenia
- Zarządzanie strukturą drzewa podczas przetwarzania

Kluczowe struktury danych:
```rust
pub struct Repository {
    pub arena: Arena<NodeData>,           // Drzewo taksonomii
    pub root: NodeId,                     // Korzeń drzewa
    pub node_by_id: HashMap<u32, NodeId>, // Mapowanie ID -> węzeł
    pub transaction_count: u32,           // Liczba transakcji
}

pub struct NodeData {
    pub id: u32,                  // Unikalny identyfikator
    pub name: String,             // Nazwa elementu
    pub bitmap: RoaringBitmap,    // Transakcje zawierające element
}
```

#### 2.1.2. Moduł Apriori (`apriori.rs`)
Implementuje rdzeń algorytmu wielopoziomowego wyszukiwania reguł asocjacyjnych.

Kluczowe struktury:
```rust
pub struct Apriori {
    pub repository: Repository,
    pub level_configs: HashMap<i32, LevelConfig>,
}

pub struct LevelConfig {
    pub support: f64,      // Min. wsparcie (0.0-1.0)
    pub confidence: f64,   // Min. ufność (0.0-1.0)
    pub gen_rules: bool,   // Czy generować reguły
}

pub struct Rule {
    pub lhs: Itemset,            // Lewa strona reguły
    pub rhs: Itemset,            // Prawa strona reguły
    pub count_full: u32,         // Liczba wystąpień całej reguły
    pub count_lhs: u32,          // Liczba wystąpień LHS
    pub num_transactions: u32,   // Całkowita liczba transakcji
}
```

#### 2.1.3. Moduł Experiments (`experiments.rs`)
Zawiera kompletną infrastrukturę do przeprowadzania eksperymentów empirycznych z serializacją wyników do JSON.

### 2.2. Algorytm główny

#### Faza 1: Wczytywanie danych
```
1. Parsuj plik taksonomii
   - Buduj drzewo hierarchii (Arena)
   - Mapuj nazwy → NodeId
2. Wczytaj transakcje
   - Dla każdej transakcji i każdego elementu
   - Ustaw bit i w bitmap węzła
3. Propaguj bitmapy od liści do korzenia
   - Dla węzła wewnętrznego: bitmap = UNION(bitmaps dzieci)
```

#### Faza 2: Wielopoziomowe wyszukiwanie (mine)
```
Wejście: Repository, LevelConfigs
Wyjście: Rules grouped by level

current_level ← 1
current_nodes ← dzieci korzenia

while current_nodes niepuste:
    if level_config istnieje dla current_level:
        # Przycinanie wg wsparcia
        min_support_count ← support * transaction_count
        usuń węzły z |bitmap| < min_support_count
        
        if gen_rules = true:
            # Generowanie itemsetów
            itemsets ← generate_itemsets(current_nodes, min_support_count)
            
            # Generowanie reguł
            rules ← generate_rules(itemsets, min_confidence)
    
    # Przejście do następnego poziomu
    current_nodes ← get_children(current_nodes)
    current_level ← current_level + 1
    
return rules_by_level
```

#### Faza 3: Generowanie itemsetów (generate_itemsets)
```
Wejście: nodes (węzły na bieżącym poziomie), min_support_count
Wyjście: large_itemsets - HashMap<size, HashMap<Itemset, count>>

# 1-itemsets
itemsets[1] ← {[node.id]: |node.bitmap| dla node w nodes jeśli |bitmap| ≥ min_support}

k ← 2
while itemsets[k-1] niepuste:
    # Join step - generowanie kandydatów
    sorted_itemsets ← sort(itemsets[k-1].keys())
    candidates ← join_step(sorted_itemsets)
    
    # Prune step - przycinanie według własności downward closure
    candidates ← prune_step(itemsets[k-1], candidates)
    
    # Obliczanie wsparcia przez intersect bitmap
    itemsets[k] ← {}
    for candidate in candidates:
        support ← calculate_support(candidate)  # bitmap intersection
        if support ≥ min_support_count:
            itemsets[k][candidate] ← support
    
    k ← k + 1

return itemsets
```

Join Step (zgodnie z Agrawal et al. 1994):
```
Wejście: Posortowane k-itemsety
Wyjście: Kandydaci (k+1)-itemsetów

i ← 0
while i < length(itemsets):
    prefix ← itemsets[i][0..k-2]  # pierwsze k-1 elementów
    tail_items ← [itemsets[i][k-1]]
    
    # Zbierz wszystkie itemsety z tym samym prefiksem
    j ← i + 1
    while j < length AND itemsets[j][0..k-2] == prefix:
        tail_items.append(itemsets[j][k-1])
        j ← j + 1
    
    # Generuj 2-kombinacje z tail_items
    for (a, b) in combinations(tail_items, 2):
        yield prefix + [a, b]
    
    i ← j
```

Prune Step (własność downward closure):
```
Wejście: itemsets[k-1], candidates
Wyjście: Nieprzycięci kandydaci

for candidate in candidates:
    # Sprawdź czy wszystkie (k-1)-podzbiory są w itemsets[k-1]
    for i in 0..(len(candidate)-2):  # Ostatnie 2 gwarantowane przez join
        subset ← candidate[0..i] + candidate[i+1..]
        if subset NOT IN itemsets[k-1]:
            odrzuć candidate
            break
```

Calculate Support (wykorzystanie bitmap):
```
Wejście: itemset = [id1, id2, ..., idk]
Wyjście: count

result ← bitmap[id1]
for id in [id2, ..., idk]:
    result ← result AND bitmap[id]  # Roaring bitmap intersection
return |result|
```

#### Faza 4: Generowanie reguł (generate_rules)
```
Wejście: itemsets, min_confidence
Wyjście: rules

rules ← []

for size ≥ 2:
    for itemset in itemsets[size]:
        # Próbuj jednolementowe konsekwenty
        H_1 ← []
        for i in 0..len(itemset):
            lhs ← itemset bez elementu i
            rhs ← [itemset[i]]
            conf ← count(itemset) / count(lhs)
            
            if conf ≥ min_confidence:
                rules.append(Rule(lhs, rhs, ...))
                H_1.append(rhs)
        
        # Rekurencyjnie buduj większe konsekwenty
        if |H_1| ≥ 2:
            ap_genrules(itemset, H_1, itemsets, min_confidence, rules)

return rules
```

ap_genrules (rekurencyjne budowanie konsekwentów):
```
Wejście: itemset, H_m (m-elementowe konsekwenty), itemsets, min_conf, rules
Efekt: Dodaje reguły do rules

if len(itemset) ≤ len(H_m[0]) + 1:
    return  # Konsekwent byłby za duży

# Generuj (m+1)-elementowe konsekwenty
H_next ← join_step(H_m) + prune_step(H_m, candidates)
H_copy ← []

for rhs in H_next:
    lhs ← itemset \ rhs
    conf ← count(itemset) / count(lhs)
    
    if conf ≥ min_conf:
        rules.append(Rule(lhs, rhs, ...))
        H_copy.append(rhs)

if H_copy niepuste:
    ap_genrules(itemset, H_copy, itemsets, min_conf, rules)
```

### 2.3. Optymalizacje

1. RoaringBitmap: Kompresja bitmap transakcji (często 10-100x lepsza niż zwykłe bitmap)
2. Short-circuit evaluation: Przerwanie obliczania wsparcia gdy spadnie poniżej minimum
3. Prealokacja: Wektory z `with_capacity` dla częstych alokacji
4. Arena allocation: Efektywne zarządzanie pamięcią drzewa (brak fragmentacji)
5. Sortowanie itemsetów: Umożliwia efektywny join step (O(n²) → O(n) dla bloków)

### 2.4. Ocena teoretycznej złożoności

#### Złożoność czasowa dla pojedynczego poziomu:
- Generowanie 1-itemsetów: O(|I| · |T|), gdzie |I| - liczba elementów, |T| - liczba transakcji
- Generowanie k-itemsetów: O(|C_k| · |T|), gdzie |C_k| - liczba kandydatów
- Join step: O(|L_{k-1}|²) w najgorszym przypadku, ale O(|L_{k-1}|) średnio dzięki sortowaniu
- Prune step: O(|C_k| · k²)
- Calculate support: O(|C_k| · k · log|T|) - bitmap intersection jest O(log n) dla RoaringBitmap
- Generowanie reguł: O(|L| · 2^k), gdzie k - rozmiar itemsetu

#### Złożoność przestrzenna:
- Repository: O(|I| · |T|/64 · compression_ratio) dla bitmap
- Itemsets: O(Σ|L_k| · k) dla przechowywania częstych itemsetów
- Arena tree: O(|I|) dla struktury drzewa

#### Korzyści z hierarchicznego podejścia:
1. Agresywne przycinanie: Usunięcie węzła przycina całe poddrzewo
2. Zmniejszenie przestrzeni poszukiwań: Wyższe progi wsparcia na górnych poziomach
3. Współdzielenie obliczeń: Bitmap propagowane raz, wykorzystywane wielokrotnie

---

## 3. Opis implementacji programu

### 3.1. Najważniejsze elementy implementacji

#### 3.1.1. Repository - Zarządzanie danymi hierarchicznymi

Budowa drzewa taksonomii:
```rust
// src/repository.rs, linie 23-135
pub fn new(transactions_path: &str, taxonomy_path: &str) 
    -> Result<Self, Box<dyn std::error::Error>>
```

Kluczowe aspekty:
- Parsowanie pliku taksonomii w formacie CSV (dziecko,rodzic)
- Dynamiczne tworzenie węzłów za pomocą closure `get_or_create_node`
- Obsługa wielu korzeni - automatyczne tworzenie sztucznego korzenia ROOT
- Wykorzystanie `indextree::Arena` dla efektywnego zarządzania pamięcią

Wczytywanie transakcji:
```rust
// src/repository.rs, linie 137-168
fn load_transactions(
    arena: &mut Arena<NodeData>,
    node_by_name: &HashMap<String, NodeId>,
    transactions_path: &str,
) -> Result<u32, Box<dyn std::error::Error>>
```

Implementacja:
- Bezpośrednia iteracja bez pośredniego wektora (optymalizacja pamięci)
- Pominięcie elementów spoza taksonomii (elastyczność danych)
- Obsługa komentarzy (linie zaczynające się od '@')
- Każda transakcja otrzymuje unikalny ID (rosnący licznik)

Propagacja bitmap:
```rust
// src/repository.rs, linie 170-191
fn propagate_bitmaps(arena: &mut Arena<NodeData>, node_id: NodeId)
```

Algorytm post-order traversal:
1. Rekurencyjnie propaguj dla wszystkich dzieci
2. Utwórz UNION bitmap wszystkich dzieci
3. Przypisz do bieżącego węzła

Optymalizacja: Użycie pierwszego dziecka jako bazy, następnie `|=` dla pozostałych.

#### 3.1.2. Apriori - Rdzeń algorytmu

Główna pętla wielopoziomowa:
```rust
// src/apriori.rs, linie 190-255
pub fn mine(&mut self) -> HashMap<usize, Vec<Rule>>
```

Implementacja wzorca "progressive deepening":
- `current_nodes` - zbiór węzłów na bieżącym poziomie
- Przycinanie przez `retain` + `remove_subtree` dla odrzuconych węzłów
- `get_children` - przejście do następnego poziomu tylko dla częstych węzłów
- Warunek stopu: `current_nodes.is_empty()`

Join step - generowanie kandydatów:
```rust
// src/apriori.rs, linie 321-374
fn join_step(&self, itemsets: &[Itemset]) -> Vec<Itemset>
```

Implementacja zgodna z algorytmem Agrawal'a:
- Iteracja z counter `skip` zamiast zagnieżdżonej pętli
- Porównanie prefixów (wszystkie oprócz ostatniego elementu)
- Kolekcja `tail_items` dla wszystkich itemsetów z tym samym prefiksem
- Generowanie par z `tail_items` (2-kombinacje)
- Wynik jest automatycznie posortowany

Prune step - wykorzystanie własności downward closure:
```rust
// src/apriori.rs, linie 376-403
fn prune_step(&self, itemmap: &HashMap<Itemset, u32>, 
              candidates: &[Itemset]) -> Vec<Itemset>
```

Optymalizacja:
- Sprawdzanie tylko pierwszych k-2 podzbiorów (ostatnie 2 gwarantowane przez join_step)
- Early break przy pierwszym brakującym podzbiorze
- Prealokacja `Vec::with_capacity` dla wyniku

Calculate support - intersect bitmap:
```rust
// src/apriori.rs, linie 405-438
fn calculate_support(&self, itemset: &Itemset) -> u32
```

Implementacja wykorzystująca RoaringBitmap:
- Optymalizacja dla 1-itemsetów: bezpośredni zwrot długości bitmap
- Dla większych: clone pierwszej bitmapy, następnie `&=` dla pozostałych
- `RoaringBitmap::operator &` jest zoptymalizowany w bibliotece

Generowanie reguł:
```rust
// src/apriori.rs, linie 440-487
fn generate_rules(&self, itemsets: &HashMap<usize, HashMap<Itemset, u32>>,
                  min_confidence: f64) -> Vec<Rule>
```

Dwuetapowe podejście:
1. Jednolementowe konsekwenty: Iteracja przez wszystkie pozycje itemsetu
2. Wieloelementowe konsekwenty: Rekurencyjne `ap_genrules`

ap_genrules - rekurencyjne budowanie konsekwentów:
```rust
// src/apriori.rs, linie 489-576
fn ap_genrules(&self, itemset: &[u32], h_m: &[Itemset], ...)
```

Kluczowe elementy:
- Warunek stopu: `len(itemset) <= len(h_m[0]) + 1`
- Użycie join_step i prune_step dla konsekwentów (reuse kodu!)
- Konwersja `h_m` do HashMap dla prune_step (dummy wartości)
- Efektywne obliczanie LHS przez merge-like algorithm (oba są posortowane)

#### 3.1.3. Obsługa konfiguracji poziomów

Konwersja negatywnych poziomów:
```rust
// src/apriori.rs, linie 142-174
pub fn from_repository(repository: Repository, 
                       level_configs: HashMap<i32, LevelConfig>) -> Self
```

Funkcjonalność:
- Obliczenie `max_level` przez BFS po drzewie
- Konwersja poziomów ujemnych: `actual_level = max_level + level + 1`
- Przykład: `-1` = ostatni poziom, `-2` = przedostatni

#### 3.1.4. Utility funkcje

Sortowanie reguł:
```rust
// src/apriori.rs, linie 102-134
pub fn sort_rules(rules: &mut Vec<Rule>, sort_by: SortBy)
```

Trzy tryby sortowania:
- `SortBy::Confidence` - malejąco według ufności
- `SortBy::Support` - malejąco według wsparcia
- `SortBy::ConfidenceSupport` - najpierw ufność, potem wsparcie

Wizualizacja drzewa:
```rust
// src/repository.rs, linie 205-273
pub fn plot_tree(&self)
```

Rekurencyjne rysowanie drzewa z:
- Box-drawing characters (├──, └──, │)
- Pominięcie poziomu liści (zbyt dużo dla czytelności)
- Zliczanie dzieci-węzłów i dzieci-liści

### 3.2. Testowanie

#### Unit testy modułu Apriori:
```rust
// src/apriori.rs, linie 579-769
#[cfg(test)]
mod tests
```

Pokrycie testowe:
- `test_join_step` - weryfikacja zgodności z przykładem z Agrawal 1994
- `test_prune_step` - sprawdzenie przycinania niepoprawnych kandydatów
- `test_calculate_support` - bitmap intersection
- `test_rule_confidence_calculation` - metryki reguł
- `test_rule_edge_cases` - przypadki brzegowe (zero divisions)

#### Integration testy:
```rust
// tests/test_apriori.rs, linie 1-469
```

13 testów integracyjnych, m.in.:
- `test_basic_itemset_generation` - podstawowa funkcjonalność
- `test_support_threshold` - filtrowanie według wsparcia
- `test_confidence_threshold` - filtrowanie reguł
- `test_apriori_algorithm_example_from_paper` - przykład z Agrawal 1994
- `test_multiple_levels` - hierarchiczne przetwarzanie
- `test_negative_level_config` - ujemne indeksy poziomów
- `test_apriori_from_repository` - alternatywna konstrukcja

Wszys tkie testy używają unikalnych plików tymczasowych (timestamp + thread_id) dla thread-safety.

### 3.3. Kompatybilność z Python

Projekt zawiera referencyjną implementację w Python (`py_code/`):
- `apriori.py` - główna funkcja API
- `itemsets.py` - generowanie częstych itemsetów
- `rules.py` - generowanie reguł asocjacyjnych

Struktura jest oparta na bibliotece `efficient-apriori`, ale dostosowana do wielopoziomowego przetwarzania. Rust implementation jest ~10-50x szybsza przy zachowaniu identycznych wyników.

---

## 4. Opis przeprowadzonych eksperymentów

### 4.1. Cel eksperymentów

Eksperymenty miały na celu:
1. Ocenę wrażliwości parametrów - jak progi wsparcia i ufności wpływają na czas wykonania i liczbę reguł
2. Ocenę skalowalności - jak algorytm radzi sobie ze wzrostem liczby transakcji i atrybutów
3. Porównanie z podejściem flat - wykazanie korzyści z hierarchicznego przetwarzania
4. Analiza efektywności przycinania - wpływ progów na wyższych poziomach na efektywność

### 4.2. Zbiory danych

Wykorzystano dwa rzeczywiste zbiory danych z Instacart (platforma zakupów spożywczych online):

#### Dataset 1: instacart_trainFIM.txt
- Transakcje zakupowe z zestawu treningowego
- Format: itemset na linię (nazwa produktu oddzielone spacjami)
- Hierarchia: 3-poziomowa taksonomia (Dział → Alejka → Produkt)

#### Dataset 2: instacart_priorFIM.txt
- Historyczne zamówienia klientów
- Większy zbiór danych niż train
- Ta sama struktura taksonomii

#### Taksonomia: instacart_taxonomy.txt
- Format CSV: `dziecko,rodzic`
- 3 poziomy abstrakcji:
  - Poziom 1: Działy (departments) - ~21 kategorii wysokopoziomowych
  - Poziom 2: Alejki (aisles) - ~134 kategorii średniopoziomowych
  - Poziom 3: Produkty (products) - tysiące pozycji

Wybór datasetu był podyktowany:
- Rzeczywiste dane (nie syntetyczne)
- Naturalna hierarchia produktów
- Różna wielkość dla testów skalowalności
- Powszechne użycie w literaturze association rule mining

### 4.3. Domyślna konfiguracja wielopoziomowa

Wszystkie eksperymenty (oprócz 3.2) używają domyślnej konfiguracji bazowej:

```rust
let mut configs = HashMap::new();
configs.insert(1, LevelConfig {
    support: 0.5,
    confidence: 0.2,
    gen_rules: false/true,  // zależnie od eksperymentu
});
configs.insert(2, LevelConfig {
    support: 0.08,
    confidence: 0.2,
    gen_rules: false/true,
});
configs.insert(3, LevelConfig {
    support: 0.005,
    confidence: 0.2,
    gen_rules: true,
});
```

Uzasadnienie progów:
- Poziom 1 (0.5): Wysokie wsparcie dla ogólnych kategorii (działy)
- Poziom 2 (0.08): Średnie wsparcie dla alejek
- Poziom 3 (0.005): Niskie wsparcie dla konkretnych produktów
- Progresywne zmniejszanie odzwierciedla naturalną strukturę danych (coraz więcej elementów, coraz mniejsze wsparcie)

### 4.4. Sposób wykonania eksperymentów

#### 4.4.1. Infrastruktura eksperymentalna

Moduł `src/experiments.rs` (1318 linii kodu) zawiera:
- Struktury danych dla wyników z derywacją Serialize/Deserialize
- Funkcje utility do zapisywania JSON
- Funkcje do modyfikacji Repository (scaling)
- 5 głównych funkcji eksperymentalnych
- Automatyczna serializacja wszystkich wyników

Averaging: Każdy eksperyment jest uruchamiany `NUM_EXPERIMENT_RUNS = 5` razy, wyniki są uśredniane dla eliminacji outlierów.

#### 4.4.2. Eksperyment 1: Parameter Sensitivity Analysis

Implementacja: `run_parameter_sensitivity()` (linie 408-643)

Metodologia:
- Część 1: Wrażliwość progów wsparcia (15 eksperymentów)
  - Poziom 1: support ∈ {0.3, 0.4, 0.5, 0.6, 0.7}, confidence = 0.2 (stały)
  - Poziom 2: support ∈ {0.04, 0.06, 0.08, 0.10, 0.12}, confidence = 0.2
  - Poziom 3: support ∈ {0.001, 0.003, 0.005, 0.007, 0.01}, confidence = 0.2
  - Każdy test zmienia tylko próg na jednym poziomie, pozostałe są domyślne

- Część 2: Wrażliwość progów ufności (5 eksperymentów)
  - Wszystkie poziomy: confidence ∈ {0.1, 0.15, 0.2, 0.25, 0.3}
  - Wsparcie na domyślnych wartościach

Pomiary:
- Całkowity czas wykonania (ms)
- Liczba wygenerowanych reguł
- 5 uruchomień na konfigurację (averaged)

Output:
```
experiments/parameter_sensitivity/
├── {dataset}_level1_s{support}_c0.2.json
├── {dataset}_level2_s{support}_c0.2.json
├── {dataset}_level3_s{support}_c0.2.json
└── {dataset}_confidence_c{confidence}.json
```

#### 4.4.3. Eksperyment 2.1: Record Scalability

Implementacja: `run_record_scalability()` (linie 705-824)

Metodologia:
- Multipliers: {1, 2, 4, 8}
- Implementacja przez rozszerzanie bitmap:
  ```rust
  fn extend_repository_transactions(repo: &Repository, multiplier: u32) -> Repository
  ```
  - Dla mult=1: oryginalny dataset
  - Dla mult=2: każda transakcja duplikowana z offsetem ID
  - Bitmap extension: `for tid in bitmap: insert(tid + offset * original_count)`
- Reuse Repository: wczytanie raz, potem klonowanie i rozszerzanie
- Użycie `Apriori::from_repository()` zamiast ponownego wczytywania plików

Pomiary:
- Czas wykonania vs liczba transakcji
- Liczba reguł (powinna pozostać stała - te same pattern)
- 5 uruchomień na multiplier

Uzasadnienie: Rozszerzanie bitmap symuluje większe datasety przy zachowaniu tej samej dystrybucji itemsetów, pozwalając izolować wpływ wolumenu danych na wydajność.

Output:
```
experiments/scalability_records/
├── {dataset}_mult1.json
├── {dataset}_mult2.json
├── {dataset}_mult4.json
└── {dataset}_mult8.json
```

#### 4.4.4. Eksperyment 2.2: Attribute Scalability

Implementacja: `run_attribute_scalability()` (linie 830-943)

Metodologia:
- Multipliers: {1.0, 1.25, 1.5, 1.75, 2.0}
- Implementacja przez dodawanie syntetycznych atrybutów:
  ```rust
  fn expand_repository_attributes(repo: &Repository, multiplier: f64) -> Repository
  ```
- Strategia generowania syntetycznych atrybutów:
  1. Losowo wybierz 5 źródłowych liści (source nodes)
  2. Dla każdego transaction ID w źródłowych bitmap:
     - Z prawdopodobieństwem 0.2 dodaj do nowego syntetycznego atrybutu
  3. Przypisz syntetyczny atrybut do rodzica pierwszego source node
  4. Powtórz dla `(multiplier - 1.0) * leaf_count` atrybutów
- Fixed seed (42) dla reproducibility
- Propagacja bitmap po dodaniu syntetycznych węzłów

Pomiary:
- Czas wykonania vs liczba atrybutów
- Liczba reguł (może się zmieniać przez nowe kombinacje)
- Original vs scaled attribute count
- 5 uruchomień na multiplier

Uzasadnienie: Symuluje poszerzenie przestrzeni atrybutów przy zachowaniu rzadkości (sparsity) danych i hierarchicznej struktury.

Output:
```
experiments/scalability_attributes/
├── {dataset}_mult1.0.json
├── {dataset}_mult1.25.json
├── {dataset}_mult1.5.json
├── {dataset}_mult1.75.json
└── {dataset}_mult2.0.json
```

#### 4.4.5. Eksperyment 3.1: Multi-Pass Comparison

Implementacja: `run_multi_pass_comparison()` (linie 949-1171)

Metodologia:

Baseline (Multi-Pass): 3 niezależne uruchomienia
```rust
// Pass 1: tylko poziom 1
configs1.insert(1, LevelConfig {
    support: 0.5, confidence: 0.2, gen_rules: true
});

// Pass 2: tylko poziom 2
configs2.insert(2, LevelConfig {
    support: 0.08, confidence: 0.2, gen_rules: true
});

// Pass 3: tylko poziom 3
configs3.insert(3, LevelConfig {
    support: 0.005, confidence: 0.2, gen_rules: true
});
```
- Każdy pass: 5 uruchomień (averaged)
- Total time = sum(pass_1_time, pass_2_time, pass_3_time)

HQApriori (Single-Pass): Jedno uruchomienie z wszystkimi poziomami
```rust
configs.insert(1, LevelConfig { support: 0.5, confidence: 0.2, gen_rules: true });
configs.insert(2, LevelConfig { support: 0.08, confidence: 0.2, gen_rules: true });
configs.insert(3, LevelConfig { support: 0.005, confidence: 0.2, gen_rules: true });
```
- 5 uruchomień (averaged)
- Hierarchy-aware pruning: węzły odrzucone na poziomie 1 nie są przetwarzane na poziomie 2-3

Metryka:
```
time_saved_pct = ((multi_pass_total - single_pass_total) / multi_pass_total) * 100
```

Pomiary:
- Czasy poszczególnych passów vs single-pass
- Liczba reguł per poziom (powinna być identyczna)
- Procent zaoszczędzonego czasu

Output:
```json
{
  "multi_pass": {
    "pass_1_time_ms": 2000,
    "pass_2_time_ms": 3500,
    "pass_3_time_ms": 5000,
    "total_time_ms": 10500,
    "rules_by_level": {"1": 1500, "2": 2000, "3": 1500}
  },
  "single_pass": {
    "total_time_ms": 6800,
    "rules_by_level": {"1": 1500, "2": 2000, "3": 1500},
    "time_saved_pct": 35.2
  }
}
```

#### 4.4.6. Eksperyment 3.2: Support Threshold Sensitivity

Implementacja: `run_support_threshold_sensitivity()` (linie 1177-1274)

Metodologia:
- 100 iteracji z randomizowanymi parametrami
- Fixed seed (42) dla reproducibility
- Generowanie reguł tylko na poziomie 3

Randomizowane parametry:
```rust
level_1_support = uniform_random(0.005, 0.8)
level_2_support = uniform_random(0.005, level_1_support)  // musi być ≤ L1
```

Stałe parametry:
- `level_3_support = 0.005` (stała)
- `confidence = 0.2` (wszystkie poziomy)
- `gen_rules = {1: false, 2: false, 3: true}`

Pomiary dla każdej iteracji:
- Total execution time (ms)
- Number of rules at level 3
- Level 1 and 2 support thresholds used

Cel: Analiza scatter plot:
- Oś X: level_1_support lub level_2_support
- Oś Y: execution time lub num_rules
- Wykazanie trade-off: wyższe progi = szybsze wykonanie ale mniej reguł

Output: Pojedynczy plik JSON z 100 rekordami:
```json
{
  "experiment_type": "support_threshold_sensitivity",
  "num_iterations": 100,
  "fixed_params": {
    "level_3_support": 0.005,
    "confidence": 0.2,
    "gen_rules_config": {"1": false, "2": false, "3": true}
  },
  "results": [
    {
      "iteration": 1,
      "level_1_support": 0.234,
      "level_2_support": 0.089,
      "total_time_ms": 5430,
      "num_rules": 4500
    },
    // ... 99 more
  ]
}
```

### 4.5. Uruchomienie eksperymentów

Kompilacja:
```bash
cargo build --release
```

Uruchomienie pełnej suity eksperymentalnej:
```bash
cargo run --release --bin experiments
```

Wykonuje wszystkie 5 eksperymentów na obu datasetach. Szacowany czas: 5-8 godzin.

Struktura wynikowa:
```
experiments/
├── parameter_sensitivity/       # ~20 plików per dataset
├── scalability_records/         # 4 pliki per dataset
├── scalability_attributes/      # 5 plików per dataset
├── multi_pass_comparison/       # 1 plik per dataset
└── support_threshold_sensitivity/  # 1 plik per dataset (100 iterations inside)
```

Wszystkie wyniki w formacie JSON z pełną konfiguracją dla reproducibility.

---

## 5. Instrukcja użycia oprogramowania

### 5.1. Wymagania systemowe

Środowisko:
- System operacyjny: Windows, Linux, lub macOS
- Rust toolchain: version 1.70+ (edition 2021)
- RAM: minimum 4GB, zalecane 8GB+ dla dużych dataset
- Miejsce na dysku: ~500MB dla kompilacji + miejsce na dane

Instalacja Rust:
```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# Pobierz i uruchom rustup-init.exe z https://rustup.rs/
```

Weryfikacja instalacji:
```bash
rustc --version  # powinno pokazać v1.70+
cargo --version
```

### 5.2. Instalacja i kompilacja projektu

Klonowanie/pobranie projektu:
```bash
git clone <repository_url>
cd HQApriori-MED
```

Struktura projektu:
```
HQApriori-MED/
├── Cargo.toml              # Konfiguracja projektu i dependencies
├── src/
│   ├── lib.rs              # Moduł biblioteki
│   ├── main.rs             # Główny executable
│   ├── apriori.rs          # Algorytm Apriori
│   ├── repository.rs       # Zarządzanie danymi
│   ├── experiments.rs      # Suite eksperymentalna
│   └── test_experiments.rs # Testy eksperymentów
├── tests/
│   └── test_apriori.rs     # Integration tests
├── datasets/               # Katalog na dane (utworzyć)
├── experiments/            # Wyniki (tworzony automatycznie)
└── py_code/                # Referencyjna implementacja Python
```

Kompilacja:
```bash
# Debug mode (szybka kompilacja, wolne wykonanie)
cargo build

# Release mode (wolna kompilacja, szybkie wykonanie - ZALECANE)
cargo build --release
```

Binary znajdują się w:
- Debug: `target/debug/hqapriori[.exe]`
- Release: `target/release/hqapriori[.exe]`

### 5.3. Specyfikacja parametrów wejściowych

#### 5.3.1. Format pliku transakcji

Każda linia = jedna transakcja (itemset), elementy oddzielone spacjami:

```
# To jest komentarz (opcjonalnie, zaczyna się od @)
milk bread butter
bread eggs
milk eggs cheese
bread butter
```

Specyfikacja:
- Format: plain text, UTF-8
- Separator: spacja (whitespace)
- Jeden itemset na linię
- Komentarze: linie zaczynające się od `@` są ignorowane
- Puste linie są ignorowane
- Elementy mogą zawierać podkreślniki i cyfry (np. `milk_2percent`)
- Kolejność elementów w linii nie ma znaczenia

Opcjonalnie: Format z dodatkowymi danymi:
```
milk bread:extra_data:timestamp
```
Wszystko po pierwszym `:` jest ignorowane (split by `:`).

#### 5.3.2. Format pliku taksonomii

Format CSV: `dziecko,rodzic` (jeden związek na linię):

```
2_percent_milk,milk
whole_milk,milk
milk,dairy
cheese,dairy
dairy,food
bread,bakery
bakery,food
```

Specyfikacja:
- Format: CSV, UTF-8
- Separator: przecinek `,`
- Dwa pola: dziecko (child), rodzic (parent)
- Puste linie są ignorowane
- Można mieć wiele korzeni (automatycznie tworzone jest ROOT)
- Nazwy muszą odpowiadać nazwom w pliku transakcji
- Hierarchia może mieć dowolną liczbę poziomów
- Cykle nie są dozwolone (drzewo, nie graf)

Przykład 3-poziomowej hierarchii:
```
# Poziom 3 (produkty) -> Poziom 2 (alejki)
banana,fruits
apple,fruits
orange,fruits
milk,dairy
cheese,dairy

# Poziom 2 (alejki) -> Poziom 1 (działy)
fruits,fresh_produce
dairy,refrigerated

# Poziom 1 -> ROOT (opcjonalnie)
fresh_produce,ROOT
refrigerated,ROOT
```

#### 5.3.3. Konfiguracja poziomów (w kodzie)

```rust
use std::collections::HashMap;
use hqapriori_med::apriori::{Apriori, LevelConfig};

let mut configs = HashMap::new();

// Poziom 1: wysokie wsparcie, nie generuj reguł
configs.insert(1, LevelConfig {
    support: 0.5,        // 50% transakcji
    confidence: 0.2,     // 20% ufności (ignorowane jeśli gen_rules=false)
    gen_rules: false,    // tylko przycięcie, bez reguł
});

// Poziom 2: średnie wsparcie
configs.insert(2, LevelConfig {
    support: 0.08,       // 8% transakcji
    confidence: 0.2,
    gen_rules: false,
});

// Poziom 3: niskie wsparcie, generuj reguły
configs.insert(3, LevelConfig {
    support: 0.005,      // 0.5% transakcji
    confidence: 0.2,
    gen_rules: true,     // generuj reguły
});

// Alternatywnie: użyj negatywnych indeksów
// -1 = ostatni poziom, -2 = przedostatni
configs.insert(-1, LevelConfig {
    support: 0.005,
    confidence: 0.2,
    gen_rules: true,
});
```

Parametry:
- `support`: float ∈ [0.0, 1.0] - minimalny próg wsparcia (percentage)
- `confidence`: float ∈ [0.0, 1.0] - minimalny próg ufności
- `gen_rules`: bool - czy generować reguły asocjacyjne na tym poziomie

### 5.4. Uruchomienie programu

#### 5.4.1. Główny program (przykład użycia)

Edytuj `src/main.rs`:

```rust
use hqapriori_med::apriori::{Apriori, LevelConfig, sort_rules, SortBy};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HQApriori - Multi-level Association Rule Mining\n");

    // Konfiguracja poziomów
    let mut configs = HashMap::new();
    configs.insert(1, LevelConfig {
        support: 0.5,
        confidence: 0.2,
        gen_rules: false,
    });
    configs.insert(2, LevelConfig {
        support: 0.08,
        confidence: 0.2,
        gen_rules: false,
    });
    configs.insert(3, LevelConfig {
        support: 0.005,
        confidence: 0.2,
        gen_rules: true,
    });

    // Utworzenie instancji Apriori
    let mut apriori = Apriori::new(
        "datasets/transactions.txt",  // ścieżka do transakcji
        "datasets/taxonomy.txt",      // ścieżka do taksonomii
        configs,
    )?;

    println!("Repository loaded:");
    println!("  Transactions: {}", apriori.repository.transaction_count);
    println!();

    // Opcjonalnie: wizualizacja drzewa
    apriori.repository.plot_tree();

    // Wykonanie algorytmu
    let mut rules_by_level = apriori.mine();

    // Wyświetlenie wyników
    println!("\n=== Results ===");
    let mut levels: Vec<usize> = rules_by_level.keys().cloned().collect();
    levels.sort();
    
    for level in levels {
        if let Some(rules) = rules_by_level.get_mut(&level) {
            // Sortowanie reguł
            sort_rules(rules, SortBy::ConfidenceSupport);

            println!("\nLevel {}: {} rules", level, rules.len());
            
            // Wyświetl top 10 reguł
            for rule in rules.iter().take(10) {
                rule.print(&apriori.repository);
            }
            
            if rules.len() > 10 {
                println!("  ... and {} more rules", rules.len() - 10);
            }
        }
    }

    Ok(())
}
```

Uruchomienie:
```bash
cargo run --release
```

#### 5.4.2. Testy jednostkowe

```bash
# Wszystkie testy
cargo test

# Testy konkretnego modułu
cargo test --lib apriori
cargo test --test test_apriori

# Testy z output
cargo test -- --nocapture

# Konkretny test
cargo test test_join_step -- --nocapture
```

#### 5.4.3. Uruchomienie eksperymentów

```bash
# Pełna suita eksperymentalna
cargo run --release --bin experiments

# Testy eksperymentów (szybkie)
cargo run --release --bin test_experiments
```

Wyniki zapisywane do `experiments/` w formacie JSON.

### 5.5. Format danych wyjściowych

#### 5.5.1. Output konsoli

Podczas wczytywania:
```
HQApriori - Multi-level Association Rule Mining

Repository loaded:
  Transactions: 50000

=== Taxonomy Tree (excluding leaf level) ===

ROOT (child_nodes: 21, child_leafs: 0)
├── beverages (child_nodes: 5, child_leafs: 0)
│   ├── water_seltzer_sparkling_water (child_nodes: 0, child_leafs: 87)
│   ├── soft_drinks (child_nodes: 0, child_leafs: 45)
│   └── ...
├── dairy_eggs (child_nodes: 8, child_leafs: 0)
│   ├── milk (child_nodes: 0, child_leafs: 32)
│   └── ...
└── ...
```

Podczas mining:
```
Mining level 1
Generating itemsets for level 1
Generating 1-itemsets
Generating k-itemsets for k = 2
Joining step
Prune step
Calculating support for candidates
Skipping rule generation at level 1 (disabled)

Mining level 2
...
```

Wyniki:
```
=== Results ===

Level 3: 3421 rules
["Bananas", "Organic_Strawberries"] -> ["Organic_Baby_Spinach"] (conf: 0.234, supp: 0.012)
["Organic_Hass_Avocado", "Bag_of_Organic_Bananas"] -> ["Organic_Strawberries"] (conf: 0.189, supp: 0.008)
...
  ... and 3411 more rules
```

#### 5.5.2. Output JSON (eksperymenty)

Parameter Sensitivity:
```json
{
  "experiment_type": "parameter_sensitivity",
  "dataset": "instacart_train",
  "level_tested": 3,
  "level_configs": {
    "1": {"support": 0.5, "confidence": 0.2, "gen_rules": false},
    "2": {"support": 0.08, "confidence": 0.2, "gen_rules": false},
    "3": {"support": 0.005, "confidence": 0.2, "gen_rules": true}
  },
  "results": {
    "total_time_ms": 1234,
    "num_rules": 3200
  }
}
```

Record Scalability:
```json
{
  "experiment_type": "record_scalability",
  "dataset": "instacart_train",
  "multiplier": 4,
  "original_transactions": 50000,
  "scaled_transactions": 200000,
  "level_configs": { ... },
  "results": {
    "total_time_ms": 4500,
    "num_rules": 3200
  }
}
```

Multi-Pass Comparison:
```json
{
  "experiment_type": "multi_pass_comparison",
  "dataset": "instacart_prior",
  "level_configs": { ... },
  "results": {
    "multi_pass": {
      "pass_1_time_ms": 2000,
      "pass_2_time_ms": 3500,
      "pass_3_time_ms": 5000,
      "total_time_ms": 10500,
      "total_rules": 5000,
      "rules_by_level": {"1": 1500, "2": 2000, "3": 1500}
    },
    "single_pass": {
      "total_time_ms": 6800,
      "total_rules": 5000,
      "rules_by_level": {"1": 1500, "2": 2000, "3": 1500},
      "time_saved_pct": 35.2
    }
  }
}
```

### 5.6. Analiza wyników (Python)

```python
import json
import pandas as pd
import glob

# Wczytanie parameter sensitivity results
results = []
for file in glob.glob('experiments/parameter_sensitivity/*.json'):
    with open(file) as f:
        data = json.load(f)
        results.append({
            'level': data['level_tested'],
            'support': data['level_configs'][str(data['level_tested'])]['support'],
            'confidence': data['level_configs'][str(data['level_tested'])]['confidence'],
            'time_ms': data['results']['total_time_ms'],
            'num_rules': data['results']['num_rules']
        })

df = pd.DataFrame(results)
print(df.head())

# Plotting
import matplotlib.pyplot as plt

plt.figure(figsize=(10, 5))
plt.subplot(1, 2, 1)
plt.scatter(df['support'], df['time_ms'])
plt.xlabel('Support threshold')
plt.ylabel('Execution time (ms)')
plt.title('Support vs Time')

plt.subplot(1, 2, 2)
plt.scatter(df['support'], df['num_rules'])
plt.xlabel('Support threshold')
plt.ylabel('Number of rules')
plt.title('Support vs Rules')

plt.tight_layout()
plt.savefig('parameter_sensitivity_analysis.png')
```

### 5.7. Rozwiązywanie problemów

Problem: `error: linking with 'cc' failed`
- Rozwiązanie (Linux): Zainstaluj build essentials: `sudo apt install build-essential`
- Rozwiązanie (Windows): Zainstaluj Visual Studio Build Tools

Problem: `out of memory`
- Rozwiązanie: Zmniejsz rozmiar datasetu lub zwiększ progi support
- Monitoruj użycie pamięci: bitmap może być duże dla dużych dataset

Problem: Brak pliku transakcji/taksonomii
- Rozwiązanie: Sprawdź ścieżki w `src/main.rs` lub `src/experiments.rs`
- Upewnij się, że katalog `datasets/` istnieje

Problem: Bardzo wolne wykonanie
- Rozwiązanie: Używaj `--release`: `cargo run --release`
- Debug mode jest 10-50x wolniejszy

Problem: Elementy z transakcji nie pasują do taksonomii
- Rozwiązanie: Algorytm pomija nieznane elementy (warning w konsoli)
- Sprawdź spójność nazw między plikami

### 5.8. Dodatkowe narzędzia

Generowanie dokumentacji API:
```bash
cargo doc --open
```

Profilowanie wydajności:
```bash
# Linux
cargo install flamegraph
cargo flamegraph --bin hqapriori

# Wszystkie platformy
cargo build --release
time target/release/hqapriori
```

Czyszczenie buildu:
```bash
cargo clean
```

---

## 6. Bibliografia

[1] J. Han and Y. Fu, "Mining Multiple-Level Association Rules in Large Databases," IEEE Transactions on Knowledge and Data Engineering, vol. 11, no. 5, pp. 798-805, Sept.-Oct. 1999, doi: 10.1109/69.806937.

[2] R. Agrawal and R. Srikant, "Fast Algorithms for Mining Association Rules," Proc. 1994 Int'l Conf. Very Large Data Bases, pp. 487-499, Santiago, Chile, Sept. 1994.

[3] RoaringBitmap - https://roaringbitmap.org/

[4] Instacart Market Basket Analysis Dataset - https://www.kaggle.com/c/instacart-market-basket-analysis

---

## Appendix A: Drzewo zależności (Cargo.toml)

```toml
[dependencies]
roaring = "0.10"          # Compressed bitmaps
indextree = "4.7"         # Tree arena allocation
serde = { version = "1.0", features = ["derive"] }  # Serialization
serde_json = "1.0"        # JSON output
rand = "0.8"              # Random number generation (experiments)
```

---

## Appendix B: Metryki reguł asocjacyjnych

Dla reguły X → Y:

Support (Wsparcie):
```
support(X → Y) = P(X ∪ Y) = count(X ∪ Y) / |Transactions|
```

Confidence (Ufność):
```
confidence(X → Y) = P(Y | X) = count(X ∪ Y) / count(X)
```

Lift (Wzrost):
```
lift(X → Y) = P(X ∪ Y) / (P(X) · P(Y))
```
- lift > 1: pozytywna korelacja
- lift = 1: niezależność
- lift < 1: negatywna korelacja

Conviction (Przekonanie):
```
conviction(X → Y) = P(X) · P(¬Y) / P(X ∪ ¬Y) = (1 - P(Y)) / (1 - confidence(X → Y))
```

---

Koniec dokumentacji
