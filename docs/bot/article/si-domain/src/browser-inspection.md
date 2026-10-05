# Browser pixel inspection

2026-10-05 UTC. Browser: dot cloud Chrome, using normal HTTPS GitHub file views.

At commit a159f0a370670817dab100203aef001f2ceda77a, all four completed PNGs were opened and screenshots inspected after scrolling to show the entire image. Hero, infographic, illustration and graph displayed correctly with Japanese labels and no missing artwork. The illustration initially had not loaded; revisiting once and observing the page confirmed the image.

- https://github.com/ishizakahiroshi/doxguard/blob/a159f0a370670817dab100203aef001f2ceda77a/docs/bot/article/si-domain/01_hero.png
- https://github.com/ishizakahiroshi/doxguard/blob/a159f0a370670817dab100203aef001f2ceda77a/docs/bot/article/si-domain/02_infographic.png
- https://github.com/ishizakahiroshi/doxguard/blob/a159f0a370670817dab100203aef001f2ceda77a/docs/bot/article/si-domain/03_illustration.png
- https://github.com/ishizakahiroshi/doxguard/blob/a159f0a370670817dab100203aef001f2ceda77a/docs/bot/article/si-domain/04_fig.png

A subsequent infographic footer copy edit removed the repository-specific SOURCES.md reference from the public image; it was regenerated and pixel-inspected locally. Source numbers, geometry and other images did not change.

This verifies the PNG display, not HTML browser overflow. The attempted local file URL was rejected by browser URL policy. No bypass was attempted. The HTML labels were laid out with PyMuPDF HTML Story and checked for box fit; browser CSS rendering remains untested.
