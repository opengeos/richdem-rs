# Publications & Credits

`richdem-rs` is a pure-Rust reimplementation of [**RichDEM**](https://github.com/r-barnes/richdem), originally designed, authored, and published by [**Richard Barnes**](https://github.com/r-barnes) (`rbarnes@umn.edu`). All credit for the library architecture, hydrologic algorithms, data structures, and test suite belongs to Richard Barnes and the original algorithm authors listed below.

> **Disclaimer:** This repository was created using **Google Gemini 4 Neon**.

## How to Cite

Please cite **RichDEM** and the relevant algorithm publications when using `richdem-rs` in research:

- **RichDEM:** Barnes, R., 2016. *RichDEM: Terrain Analysis Software*. <https://github.com/r-barnes/richdem>
- **Priority-Flood:** Barnes, R., Lehman, C., Mulla, D., 2014. Priority-flood: An optimal depression-filling and watershed-labeling algorithm for digital elevation models. *Computers & Geosciences* 62, 117–127. doi:[10.1016/j.cageo.2013.04.024](https://doi.org/10.1016/j.cageo.2013.04.024)
- **Flat Resolution:** Barnes, R., Lehman, C., Mulla, D., 2014. An efficient assignment of drainage direction over flat surfaces in raster digital elevation models. *Computers & Geosciences* 62, 128–135. doi:[10.1016/j.cageo.2013.01.009](https://doi.org/10.1016/j.cageo.2013.01.009)
- **Depression Hierarchies:** Barnes, R., Callaghan, K.L., Wickert, A.D., 2020. Computing water flow through complex landscapes - Part 2: Finding hierarchies in depressions and morphological segmentations. *Earth Surface Dynamics* 8, 431–445. doi:[10.5194/esurf-8-431-2020](https://doi.org/10.5194/esurf-8-431-2020)
- **Fill-Spill-Merge:** Barnes, R., Callaghan, K.L., Wickert, A.D., 2021. Computing water flow through complex landscapes - Part 3: Fill-Spill-Merge: Flow routing in depression hierarchies. *Earth Surface Dynamics* 9, 105–121. doi:[10.5194/esurf-9-105-2021](https://doi.org/10.5194/esurf-9-105-2021)
- **Zhou2016 Priority-Flood:** Zhou, G., Sun, Z., Fu, S., 2016. An efficient variant of the Priority-Flood algorithm for filling depressions in raster digital elevation models. *Computers & Geosciences* 90, 87–96. doi:[10.1016/j.cageo.2016.02.021](https://doi.org/10.1016/j.cageo.2016.02.021)
- **Wei2018 Priority-Flood:** Wei, H., Zhou, G., Fu, S., 2018. Efficient Priority-Flood depression filling in raster digital elevation models. *International Journal of Digital Earth* 12, 415–427. doi:[10.1080/17538947.2018.1429503](https://doi.org/10.1080/17538947.2018.1429503)
- **Depression Breaching:** Lindsay, J.B., 2016. Efficient hybrid breaching-filling sink removal methods for flow path enforcement in digital elevation models. *Hydrological Processes* 30, 846–857. doi:[10.1002/hyp.10648](https://doi.org/10.1002/hyp.10648)
- **D-Infinity:** Tarboton, D.G., 1997. A new method for the determination of flow directions and upslope areas in grid digital elevation models. *Water Resources Research* 33, 309–319.
- **D8 / D4:** O'Callaghan, J.F., Mark, D.M., 1984. The extraction of drainage networks from digital elevation data. *CVGIP* 28, 323–344.
- **Rho8 / Rho4:** Fairfield, J., Leymarie, P., 1991. Drainage networks from grid digital elevation models. *Water Resources Research* 27, 709–717.
- **Multiple Flow Direction:** Quinn et al. (1991); Freeman (1991); Holmgren (1994).
- **Slope, Aspect, Curvature & Surface Area:** Horn (1981); Zevenbergen & Thorne (1987); Garbrecht & Martz (1997); Jenness (2004).
- **Raster I/O:** [`terrio`](https://terrio.dev) (MIT / MPL-2.0 / Apache-2.0).

## License

`richdem-rs` is licensed under the **GNU General Public License v3.0 (`GPL-3.0-only`)**, matching upstream [RichDEM](https://github.com/r-barnes/richdem). See [LICENSE](https://github.com/opengeos/richdem-rs/blob/main/LICENSE) and [NOTICE](https://github.com/opengeos/richdem-rs/blob/main/NOTICE) for full details.
