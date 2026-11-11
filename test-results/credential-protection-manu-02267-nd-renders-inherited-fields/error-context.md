# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: credential-protection.spec.ts >> manual v3 registration keeps secrets out of responses and renders inherited fields
- Location: frontend\tests\credential-protection.spec.ts:62:5

# Error details

```
Test timeout of 30000ms exceeded.
```

# Page snapshot

```yaml
- generic [ref=e1]:
  - navigation [ref=e2]:
    - link "TracePulse" [ref=e3] [cursor=pointer]:
      - /url: /
    - generic [ref=e6]:
      - link "Dashboard" [ref=e7] [cursor=pointer]:
        - /url: /
      - link "Discovery" [ref=e8] [cursor=pointer]:
        - /url: /discovery
      - link "Diagnostics" [ref=e9] [cursor=pointer]:
        - /url: /diagnostics
      - link "Settings" [ref=e10] [cursor=pointer]:
        - /url: /settings
    - generic [ref=e11]:
      - generic [ref=e12]:
        - generic [ref=e13]: Theme
        - combobox "Theme" [ref=e14]:
          - option "Dark" [selected]
          - option "Light"
      - generic [ref=e15]:
        - generic [ref=e16]: Language
        - combobox "Language" [ref=e17]:
          - option "English" [selected]
          - option "Japanese"
  - main [ref=e18]:
    - heading "Device Discovery" [level=1] [ref=e19]
    - generic [ref=e20]:
      - generic [ref=e21]:
        - generic [ref=e22]: Community
        - textbox "Community" [ref=e23]:
          - /placeholder: public
          - text: public
      - generic [ref=e24]:
        - generic [ref=e25]:
          - generic [ref=e26]:
            - generic [ref=e27]: CIDR Range
            - textbox "CIDR Range" [ref=e28]:
              - /placeholder: 192.168.1.0/24
          - button "Scan" [ref=e29] [cursor=pointer]
        - generic [ref=e30]:
          - generic [ref=e31]:
            - generic [ref=e32]: Seed Device IP (LLDP/CDP)
            - textbox "Seed Device IP (LLDP/CDP)" [ref=e33]:
              - /placeholder: 192.168.11.23
          - button "Draw Topology" [ref=e34] [cursor=pointer]
      - link "+ Add manually" [active] [ref=e37] [cursor=pointer]:
        - /url: "#"
```