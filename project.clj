(defproject fractal-explorer "0.1.0-SNAPSHOT"
  :description "Interactive fractal art generator in Clojure"
  :dependencies [[org.clojure/clojure "1.11.1"]
                 [quil "4.3.1563"]]
  :main core
  :source-paths ["src"]
  :test-paths ["test"]
  :profiles {:test {:jvm-opts ["-Djava.awt.headless=true"]
                    :aot [core]}})
