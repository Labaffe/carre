/!\ différence entre niveaux dans le sens de :
    - niveau du joueur qui se passe en augmentant son score
    - niveau dans lequel le joueur évolue

Je propose :
    - niveau du joueur -> shipLevel
    - niveau physique -> gameLevel


### TODO
[X] Améliorer la visibilité du viseur
[x] (ship)Level up par palier
[_] Ecran de passage de *shipLevel* avec choix d'un bonus (shield +1, health +1, dégat + x%, nouvel arme etc.) tu la passé la c'est que tu a validé ? la croix c'est si c'est fait. Ou alors tu pensé au  gameLevel et pas au shipLevel ?
[_] modifier l'écran de selection de niveau, chaque boss est un avis de recherche mais c'est tellement dur de faire un truc joli

### Idées (pls mettre un avis et après on le met en todo)
- Ecran de passage de niveau avec choix d'un bonus (shield +1, health +1, dégat + x%, nouvel arme etc.)
- Ajout de bonus disponibles parmis une selection après chaque boss vaincu (omg deckbuilding ?)
- shield : comme de la health mais se rerempli après chaque boss/encounter ?
- Plus d'armes, possibilités : 
    - hitscan avec long temps de recharge
    - aoe de proximité (tape sur les enemies à proximité du vaisseau)
    - missile lent mais avec une grosse boite de collision

Question : en fait ce que je proposer impliquait une certaine liberté de choix de la part du joueur au niveau des armes, mais peut être ca va pas avec la philosophie shoot em up ? Est-ce que la selection doit être automatique ? Sinon j'explique un peu plus en détails ce à quoi je pensais avec les 4 points précedent.

- Créer des niveaux visitables dans tous les sens comme dans Megaman. 
    - Oh on fait un metroidvania, ca me vas ! Mais comment on fait par rapport au scrolling permanent ?
    - Non dans Mégaman, c'est des niveaux fixes que tu peux visiter dans l'ordre, en principe tu vas tuer le boss A qui te donne une arme efficace contre le boss B mais je pense qu'on peut mixer ça a du Roguelike
    - Oui on peut. Question : est-ce que tous les gameLevel sont disponible dès le début ? Je vois deux options : (1) un rogue classique ou tu dois organiser ta run en choisissant tes gameLevel. (2) chaque gameLevel est un minirogue et la progression est conservé. Un jeu à l'ancienne quoi 
  
### Améliorations

