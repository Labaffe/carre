/!\ différence entre niveaux dans le sens de :
    - niveau du joueur qui se passe en augmentant son score
    - niveau dans lequel le joueur évolue

Je propose :
    - niveau du joueur -> shipLevel
    - niveau physique -> gameLevel

Oui tout à fait. Le joueur gagne des ShipLevel en parcourant des GameLevel dans l'ordre qu'il souhaite.

### TODO
[X] Améliorer la visibilité du viseur
[x] (ship)Level up par palier
[] Ecran de passage de *shipLevel* avec choix d'un bonus (shield +1, health +1, dégat + x%, nouvel arme etc.) tu la passé la c'est que tu a validé ? la croix c'est si c'est fait. Ou alors tu pensé au  gameLevel et pas au shipLevel ?
C'est ok pour moi ! si t'es chaud d'ajouter ce systeme c'est avec grand plaisir !
[] Créer un niveau 2
[X] modifier l'écran de selection de niveau, chaque boss est un avis de recherche mais c'est tellement dur de faire un truc joli

### Idées (pls mettre un avis et après on le met en todo)
- Ecran de passage de niveau avec choix d'un bonus (shield +1, health +1, dégat + x%, nouvel arme etc.)
- Ajout de bonus disponibles parmis une selection après chaque boss vaincu (omg deckbuilding ?)
- shield : comme de la health mais se rerempli après chaque boss/encounter ?
- Plus d'armes, possibilités : 
    - hitscan avec long temps de recharge
    - aoe de proximité (tape sur les enemies à proximité du vaisseau)
    - missile lent mais avec une grosse boite de collision

Question : en fait ce que je proposer impliquait une certaine liberté de choix de la part du joueur au niveau des armes, mais peut être ca va pas avec la philosophie shoot em up ? Est-ce que la selection doit être automatique ? Sinon j'explique un peu plus en détails ce à quoi je pensais avec les 4 points précedent.

Dans Megaman il y a un vrai intéret à aller taper un boss plutot qu'un autre car il te donne des armes spéciales. Etant donné qu'on veut faire une dimension roguelike autant garder simplement le choix de l'ordre des boss. 
A la base je me suis dis que créer des niveaux fixes (meme si on peut y ajouter du random à l'intérieur) était un objectif relativement atteignables.

- Créer des niveaux visitables dans tous les sens comme dans Megaman. 
    - Oh on fait un metroidvania, ca me vas ! Mais comment on fait par rapport au scrolling permanent ?
    - Non dans Mégaman, c'est des niveaux fixes que tu peux visiter dans l'ordre, en principe tu vas tuer le boss A qui te donne une arme efficace contre le boss B mais je pense qu'on peut mixer ça a du Roguelike
    - Oui on peut. Question : est-ce que tous les gameLevel sont disponible dès le début ? Je vois deux options : (1) un rogue classique ou tu dois organiser ta run en choisissant tes gameLevel. (2) chaque gameLevel est un minirogue et la progression est conservé. Un jeu à l'ancienne quoi.

 Dans ma tête tous les boss sont disponibles au debut. Mais cela va poser une question d'équilibrage.
 Choix :
 - Les boss sont dans un ordre précis, ils sont donc de plus en plus difficiles (plus facile a équilibrer)
 - Les boss sont tous disponibles, mais les boss ont une version 1, 2, 3, 4 suivant l'ordre dans lequel tu les rencontres (cela peut etre simplement un scale de pv/vitesse) (je pars du principe que pour l'instant on part sur 4 boss
 - Les boss sont tous disponibles mais comme dans Megaman, ils sont difficiles. L'avantage c'est que si tu galères sur un Boss tu peux essayer d'en faire un autre sens rester bloqué. 
  
### Améliorations

Les sprites doivent avoir une cohérence visuelle -> différence entre la planete en fond et le sprite du Space Invader
