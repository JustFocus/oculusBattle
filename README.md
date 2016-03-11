# JavaScript Game: [Oculus Battle!][description]

**[Check it out!][live-demo]**

[live-demo]: http://justfocus.github.io/oculusBattle/
[description]: http://justfocus.github.io/oculusBattle/


#Game Details
###Bases

- Except for the “home” locations, base locations are randomly generated each time the game is started or a new level begins.
- Randomly generated bases space themselves based on the number created and their size.
- Bases outline color changes upon selection.
- Base selection determined based on cursor location and location of bases.

###Units

- Units are spawned at random locations surrounding the base that created them.
- Upon spawning units calculate velocity vector to final target
- Units have hitbox to check for collision with target

###AI
- The AI always attacks from it’s strongest base with greater than 15 units
- Waits approx 1-2 seconds (varies) between each attack
- Determines which base to attack by using a valuation equation that scales each base value by a random amount.
-	The AI's bases generate units faster as the levels progress.


###Detailed Instructions

Oculus battle is a simplified RTS. The goal is to eliminate the enemy! You are the blue team and start on the left side of the map. The red team is the enemy and is controlled by the computer. The black team is neutral and cannot attack or generate units.

###PLAYING THE GAME:

You and your enemy both start with one base. Bases controlled by red or blue automatically generate units depending on their initial size. Bases can be selected with a mouse click. Once a base is selected you can then click another base to attack. Attacking sends half of a bases units to the second base clicked. You can also use this to reinforce your own bases. When a base goes below zero units it is taken over by the attacking team. A bases maximum size is twice its starting value.

**Recap:**
- Bases generate units over time based on starting size.
- Select a base by left clicking on it.
- Attack a base by first selecting a base you own then clicking another base.
- You can send units from bases you own to other bases you own.
- When you take over a base it will begin generating units for you.
