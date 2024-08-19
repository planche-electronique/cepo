# Serveur interface
[![](https://tokei.rs/b1/github/planche-electronique/cepo)](https://github.com/planche-electronique/cepo)
## Introduction

Ce petit serveur web fait l'interface entre le site web et le javascript de la
[planche](http://github.com:planche-electronique/eplanche). De plus, il
s'occupe de récupérer les données de ogn et les stocker.

## Installation

Il faut copier le fichier `infos.json` dans $XDG_DATA_HOME/cepo

## Crédits
Merci à OGN pour la récupération des données de vol et leur [API](https://gitlab.com/davischappins/ogn-flightbook/-/blob/master/doc/API.md).
Non affilié à la rust foundation.

## TO-DO
- before you could get "infos.json", now we should be able to send airport +
global infos on request
- refactor code to make it prettier (like the big `connection_handler` function)
- feature: each `always` set airport should have its own thread
