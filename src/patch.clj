(ns patch
  (:require
   [effects :as fx]
   [generators :as gen]
   [modulators :as mod]))

;; A patch is a signal flow graph
;; generator -> [effects] -> renderer

(defrecord Patch [generator effects modulators params gen-cache last-gen-params])

(defn create-patch
  "Create a new patch with generator, effect chain, and modulators"
  [generator effects modulators initial-params]
  (->Patch generator effects modulators initial-params nil nil))

(defn process-patch
  "Process a patch: apply modulators, generate signal, apply effects.
  Returns {:pixel-data … :params … :patch …} where :patch carries the
  updated cache so callers can store it for the next frame."
  [patch]
  (let [;; Apply modulators - they should return modified params map
        ;; ModMatrix is the only modulator that actually modifies params
        ;; Other modulators (LFO, etc) should only be used inside ModMatrix
        modulated-params (if (empty? (:modulators patch))
                           (:params patch)
                           (reduce (fn [p modulator]
                                   ;; Only ModMatrix returns modified params
                                   ;; All others are building blocks FOR ModMatrix
                                     (if (instance? modulators.ModMatrix modulator)
                                       (mod/modulate modulator p)
                                       p))  ;; Skip non-ModMatrix modulators
                                   (:params patch)
                                   (:modulators patch)))

        ;; Determine which params the generator actually reads
        relevant-keys    (gen/gen-params (:generator patch))
        current-gen-params (select-keys modulated-params relevant-keys)

        ;; Use cached pixel data when generator-relevant params haven't changed
        [pixel-data updated-patch]
        (if (= current-gen-params (:last-gen-params patch))
          [(:gen-cache patch) patch]
          (let [fresh (gen/generate (:generator patch) modulated-params)]
            [fresh (assoc patch :gen-cache fresh :last-gen-params current-gen-params)]))

        ;; Apply effect chain
        processed-data (reduce (fn [data effect]
                                 (fx/process effect data modulated-params))
                               pixel-data
                               (:effects updated-patch))]

    {:pixel-data processed-data
     :params     modulated-params
     :patch      (assoc updated-patch :params modulated-params)}))

(defn update-patch-params
  "Update patch parameters (for user interaction)"
  [patch updates]
  (update patch :params merge updates))

(defn add-effect
  "Add an effect to the end of the effect chain"
  [patch effect]
  (update patch :effects conj effect))

(defn remove-effect
  "Remove an effect at index from the chain"
  [patch index]
  (update patch :effects
          (fn [effects]
            (vec (concat (take index effects)
                         (drop (inc index) effects))))))

(defn replace-effect
  "Replace effect at index"
  [patch index new-effect]
  (update patch :effects assoc index new-effect))

(defn add-modulator
  "Add a modulator to the patch"
  [patch modulator]
  (update patch :modulators conj modulator))

(defn replace-generator
  "Swap out thgenerator"
  [patch new-generator]
  (assoc patch :generator new-generator))
