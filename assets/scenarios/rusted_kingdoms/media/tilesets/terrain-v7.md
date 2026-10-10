```mermaid
graph TD
  classDef hub fill:#fee2e2,stroke:#64748b,color:#111827;
  classDef bridge fill:#fef3c7,stroke:#64748b,color:#111827;
  classDef branch fill:#dbeafe,stroke:#64748b,color:#111827;
  classDef leaf fill:#dcfce7,stroke:#64748b,color:#111827;
  classDef bridgeEdge stroke:#f59e0b,stroke-width:3px;
  subgraph g1_Terrains_01___connected_grass["Terrains 01 - connected-grass"]
    g1_Terrains_01___connected_grass__Dirt_Roots["Dirt_Roots<br/>branch (2)"]
    class g1_Terrains_01___connected_grass__Dirt_Roots branch
    g1_Terrains_01___connected_grass__Grass["Grass<br/>hub (8)"]
    class g1_Terrains_01___connected_grass__Grass hub
    g1_Terrains_01___connected_grass__Grass_Dark["Grass_Dark<br/>branch (2)"]
    class g1_Terrains_01___connected_grass__Grass_Dark branch
    g1_Terrains_01___connected_grass__Ice["Ice<br/>branch (2)"]
    class g1_Terrains_01___connected_grass__Ice branch
    g1_Terrains_01___connected_grass__Ice_Melting["Ice_Melting<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Ice_Melting leaf
    g1_Terrains_01___connected_grass__Mudstone_Brown["Mudstone_Brown<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Mudstone_Brown leaf
    g1_Terrains_01___connected_grass__Mudstone_Gray["Mudstone_Gray<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Mudstone_Gray leaf
    g1_Terrains_01___connected_grass__Rock["Rock<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Rock leaf
    g1_Terrains_01___connected_grass__Sand["Sand<br/>branch (2)"]
    class g1_Terrains_01___connected_grass__Sand branch
    g1_Terrains_01___connected_grass__Snow_1["Snow_1<br/>bridge (3)"]
    class g1_Terrains_01___connected_grass__Snow_1 bridge
    g1_Terrains_01___connected_grass__Soil["Soil<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Soil leaf
    g1_Terrains_01___connected_grass__Stone_Tan["Stone_Tan<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Stone_Tan leaf
    g1_Terrains_01___connected_grass__Stone_White["Stone_White<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Stone_White leaf
    g1_Terrains_01___connected_grass__Water["Water<br/>hub (6)"]
    class g1_Terrains_01___connected_grass__Water hub
    g1_Terrains_01___connected_grass__Water_Deep["Water_Deep<br/>leaf (1)"]
    class g1_Terrains_01___connected_grass__Water_Deep leaf
    g1_Terrains_01___connected_grass__Water_Shallows_Dirt["Water_Shallows_Dirt<br/>bridge (3)"]
    class g1_Terrains_01___connected_grass__Water_Shallows_Dirt bridge
    g1_Terrains_01___connected_grass__Water_Shallows_Sand["Water_Shallows_Sand<br/>branch (2)"]
    class g1_Terrains_01___connected_grass__Water_Shallows_Sand branch
    g1_Terrains_01___connected_grass__Dirt_Roots ---|14| g1_Terrains_01___connected_grass__Grass
    g1_Terrains_01___connected_grass__Dirt_Roots ---|14| g1_Terrains_01___connected_grass__Grass_Dark
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Mudstone_Brown
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Mudstone_Gray
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Rock
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Soil
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Stone_Tan
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Stone_White
    g1_Terrains_01___connected_grass__Grass ---|14| g1_Terrains_01___connected_grass__Water_Shallows_Dirt
    g1_Terrains_01___connected_grass__Grass_Dark ---|14| g1_Terrains_01___connected_grass__Water_Shallows_Dirt
    g1_Terrains_01___connected_grass__Ice ---|62| g1_Terrains_01___connected_grass__Snow_1
    g1_Terrains_01___connected_grass__Ice ---|61| g1_Terrains_01___connected_grass__Water
    g1_Terrains_01___connected_grass__Ice_Melting ---|14| g1_Terrains_01___connected_grass__Snow_1
    g1_Terrains_01___connected_grass__Sand ---|62| g1_Terrains_01___connected_grass__Water
    g1_Terrains_01___connected_grass__Sand ---|62| g1_Terrains_01___connected_grass__Water_Shallows_Sand
    g1_Terrains_01___connected_grass__Snow_1 ---|61| g1_Terrains_01___connected_grass__Water
    g1_Terrains_01___connected_grass__Water ---|14| g1_Terrains_01___connected_grass__Water_Deep
    g1_Terrains_01___connected_grass__Water ---|14| g1_Terrains_01___connected_grass__Water_Shallows_Dirt
    g1_Terrains_01___connected_grass__Water ---|62| g1_Terrains_01___connected_grass__Water_Shallows_Sand
  end
  linkStyle 2 stroke:#f59e0b,stroke-width:3px;
  linkStyle 3 stroke:#f59e0b,stroke-width:3px;
  linkStyle 4 stroke:#f59e0b,stroke-width:3px;
  linkStyle 5 stroke:#f59e0b,stroke-width:3px;
  linkStyle 6 stroke:#f59e0b,stroke-width:3px;
  linkStyle 7 stroke:#f59e0b,stroke-width:3px;
  linkStyle 12 stroke:#f59e0b,stroke-width:3px;
  linkStyle 16 stroke:#f59e0b,stroke-width:3px;
  linkStyle 17 stroke:#f59e0b,stroke-width:3px;
```
