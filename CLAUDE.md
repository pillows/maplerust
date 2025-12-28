File:Placeholder

Backgrounds fill in the background to every map. You can't interact with them but you can see them. Some backgrounds are in the foreground such as clouds and they can obscure your view.

Backgrounds in the WZ Data
In the map
back
[id number]
bS - The background set that contains the background.
front - Whether the background is actually a foreground.
ani - Whether the background is animated.
no - Which background to load from the set.
f - Whether the background is flipped over the y-axis.
x - The x coordinate of the background.
y - The y coordinate of the background.
rx - Determines the parallex speed along the x-axis.
ry - Ditto for the y-axis.
type - Determines tiling and movement.
cx - Replaces the sprite's width for tiling if nonzero.
cy - Ditto for the height.
a - The alpha transparency of the background.
In the background set
back - If the background is not animated.
[the no of the background]
width - The width of the sprite.
height - The height.
origin - The sprite's offset from it's coordinates.
x - The x offset.
y - The y offset.
z - Unused.
ani - If the background is animated.
[the no of the background]
width - The width of the sprite.
height - The height.
origin - The sprite's offset from it's coordinates.
x - The x offset.
y - The y offset.
z - Unused.
a0 - The alpha at the beginning of the frame. [optional]
a1 - The alpha at the end of the frame. [optional]
delay - The delay in milliseconds before the next frame is loaded. [optional]
moveType - The type of spinning or bobbing motion it does. [optional]
1 indicates horizontal bobbing.
2 indicates vertical bobbing.
3 indicates spinning motion.
moveR - How fast the background spins.
moveP - How fast the background bobs.
moveW - How far the background bobs horizontally.
moveH - How far the background bobs vertically.
Loading Backgrounds
Iterate through all the background data in the map.
For each back, find by the background set by navigating to Map.wz/Back/[bS].
If ani is true navigate into the ani section. Otherwise navigate into the back section.
Find the background whose number is equal to the no in the map data.
Load in all the data from both the map and the background set for that back.
Calculating Position and Stuff

Keep track of the time since the last frame (delta) and also a continuous timer which never loops (tdelta). Do this in milliseconds (or convert if you feel like it).

Dealing with Parallex

If you already account for the view then calculating the parallex offset of the background (bx, by) in relation to the view (vx, vy) is fairly simple

bx = (100+rx)/100*vx;

by = (100+ry)/100*vy;

If you have to account for the view in computing the offset then you can do

bx = rx/100*vx;

by = ry/100*vy;

Uh, someone needs to check that and probably fix it.

Dealing with moveType

moveType is basically bobbing or rotation allowing for smooth motion without having to animate it all.

If moveP is nonzero then that affects the speed of the bobbing.

If moveType is 1, you have horizontal bobbing

With moveP

ax = moveW*sin(tdelta*1000*2*pi/moveP);

Without moveP

ax = moveW*sin(tdelta);

If moveType is 2, you have vertical bobbing

With moveP

ay = moveH*sin(tdelta*1000*2*pi/moveP);

Without moveP

ay = moveH*sin(tdelta);

For the sin function, replace the 2*pi with 360 if your programming language uses degrees instead of radians If moveType is 3, you have rotationimage_angle = tdelta*1000/mover*360/2/pi;

Get rid of the *360/2/pi at the end if your programming language uses radians for rotation instead of degrees.

File:Placeholder

Backgrounds fill in the background to every map. You can't interact with them but you can see them. Some backgrounds are in the foreground such as clouds and they can obscure your view.

Backgrounds in the WZ Data
In the map
back
[id number]
bS - The background set that contains the background.
front - Whether the background is actually a foreground.
ani - Whether the background is animated.
no - Which background to load from the set.
f - Whether the background is flipped over the y-axis.
x - The x coordinate of the background.
y - The y coordinate of the background.
rx - Determines the parallex speed along the x-axis.
ry - Ditto for the y-axis.
type - Determines tiling and movement.
cx - Replaces the sprite's width for tiling if nonzero.
cy - Ditto for the height.
a - The alpha transparency of the background.
In the background set
back - If the background is not animated.
[the no of the background]
width - The width of the sprite.
height - The height.
origin - The sprite's offset from it's coordinates.
x - The x offset.
y - The y offset.
z - Unused.
ani - If the background is animated.
[the no of the background]
width - The width of the sprite.
height - The height.
origin - The sprite's offset from it's coordinates.
x - The x offset.
y - The y offset.
z - Unused.
a0 - The alpha at the beginning of the frame. [optional]
a1 - The alpha at the end of the frame. [optional]
delay - The delay in milliseconds before the next frame is loaded. [optional]
moveType - The type of spinning or bobbing motion it does. [optional]
1 indicates horizontal bobbing.
2 indicates vertical bobbing.
3 indicates spinning motion.
moveR - How fast the background spins.
moveP - How fast the background bobs.
moveW - How far the background bobs horizontally.
moveH - How far the background bobs vertically.
Loading Backgrounds
Iterate through all the background data in the map.
For each back, find by the background set by navigating to Map.wz/Back/[bS].
If ani is true navigate into the ani section. Otherwise navigate into the back section.
Find the background whose number is equal to the no in the map data.
Load in all the data from both the map and the background set for that back.
Calculating Position and Stuff

Keep track of the time since the last frame (delta) and also a continuous timer which never loops (tdelta). Do this in milliseconds (or convert if you feel like it).

Dealing with Parallex

If you already account for the view then calculating the parallex offset of the background (bx, by) in relation to the view (vx, vy) is fairly simple

bx = (100+rx)/100*vx;

by = (100+ry)/100*vy;

If you have to account for the view in computing the offset then you can do

bx = rx/100*vx;

by = ry/100*vy;

Uh, someone needs to check that and probably fix it.

Dealing with moveType

moveType is basically bobbing or rotation allowing for smooth motion without having to animate it all.

If moveP is nonzero then that affects the speed of the bobbing.

If moveType is 1, you have horizontal bobbing

With moveP

ax = moveW*sin(tdelta*1000*2*pi/moveP);

Without moveP

ax = moveW*sin(tdelta);

If moveType is 2, you have vertical bobbing

With moveP

ay = moveH*sin(tdelta*1000*2*pi/moveP);

Without moveP

ay = moveH*sin(tdelta);

For the sin function, replace the 2*pi with 360 if your programming language uses degrees instead of radians If moveType is 3, you have rotationimage_angle = tdelta*1000/mover*360/2/pi;

Get rid of the *360/2/pi at the end if your programming language uses radians for rotation instead of degrees.


[[File:Ladder.png|thumb|112px|right|A grassySoil style ladder.]]
Ladders, also called ropes or more collectively ladderRope, are the things you climb on in MapleStory.
==Ladders in the wz data==
*map .img node
**ladderRope
***ID of ladder
****x - The x coordinate of the ladder
****y1 - The first y coordinate
****y2 - The second y coordinate
****l - Whether or not the ladderRope is a ladder, or otherwise a rope.
****page - The depth of the player when climbing the ladderRope
****uf - Whether or not the player cannot climb off the top of the ladderRope.

[[File:Placeholder|right|300px]]
Maps are set locations in the MapleStory world. You are '''always''' in a Map, whether it be in Henesys, or even on the Login Screen; you're always in a Map.
==Maps in the WZ Data==
===Location===
Maps can be found in [[Map.wz]] in the following location in the wz tree:
*Map.wz
**Map
***Map? (where ? is a number from 0 to 9 and is equal to the first digit of the map's id)
****[the id of your map].img
===Contents===
Inside each map is the following information:
====Standard information : These appear in each and every map.====
*info
*back
*life
*reactor
*0,1,2,3,4,5,6,7
*foothold
*ladderRope
*miniMap
*portal
*reactor
*seat

====Other information : These are less commonly found, and are normally used on maps with special features.====
*clock
*pulley
*healer
*monsterCarnival
*snowBall
*weather
*user
*BuffZone
*noSkill
*seat
*battleField
*snowMan
*mobMassacre
*swimArea
*nodeInfo
*shipObj
*coconut
*ToolTip

===Standard Information===

====Info - Contains basic info and restrictions regarding the map====
**version - Goes up when Nexon edits the map
**cloud - Whether or not there are 'mist' clouds present on the map; where the value of 0 equates to no clouds, and 1 has clouds.
**town - Whether this is a town. Affects death exp loss.
**swim - Whether or not you can 'swim' in the map, regardless of the presence of any body of water. Just like with the Cloud property; 0 means you can't swim, and 1 means you can swim.
**returnMap - If you die or use a return scroll where do you go? The mapid set here will be the destination.
**forcedReturn - If you do so much as change channels or logout you'll get sent here. The mapid set here will be the destination.
**mobRate - How fast mobs spawn.
**bgm - The background music. Music can be found in the Sound.wz
**mapMark - The mark for the map in the minimap.
**fly - Whether or not you can fly in the map. Flying is a slower and 'heavier' alternative to swimming. The value of 0 means you cannot fly, while the value of 1 means you can fly.
**noMapCmd - Map commands are not allowed to be used on these maps. /m <mapid> is disabled.
**hideMinimap - Is the minimap hidden? Where the value of 0 means it isn't, and the value of 1 means it is.
**fieldLimit - Used to check several stuffs in maps. Something like map rules. Eg: Jump, downjump, spawn pets, use skills, teleport rocks, change channel, etc.
**VRTop - The upper boundary of the map.
**VRLeft - The left boundary.
**VRBottom - And the bottom.
**VRRight - And the right.
**onFirstUserEnter - When you first come here execute this script.
**onUserEnter - Whenever you enter this map execute this script.
**moveLimit - Mobile restrictions
**entrustedShop - [Boolean] Enables Hired Merchants to be used
**personalShop - [Boolean] Enables Personal Shops to be used
**help - [String] Not sure what it does, but its in korean words in the wz files
**zakum2Hack - Zakum Jump Quest Check
**allMoveCheck - Jumping Quest check protection. Can be used to check for player movements
**allowedItem - In int forms. Only these items can be dropped in the map. (Used in Wolf/Sheep PQ)
**decHP/decMP - Amount of hp or mp to decerease at a specific <decInterval> . (Default: 10 secs)
**protectItem - decHP and decMP will NOT take effect when this item is equipped
**protectSetKey - decHP and decMP will NOT take effect when the WHOLE set is equipped. (Eg: Visitors Set)
**fs - Slipping on ice speed. (Default: 0.2)
**EscortMinTime - Time allowed to escord a mob npc. (Eg: Visitors PQ and Shammos PQ)
**reactorShuffle - [Boolean] shuffles <reactorShuffleName> upon entering map. If name does not exist, then shuffle all.
**expeditionOnly - Only players with expedition canenter this map
**partyOnly - Only players with party can enter this map
**fixedMobCapacity - Max amount of mob that can stay in the map (Used in Subway PQ and Pyramid PQ)
**createMobInterval - Intervals to spawn mob (In milliseconds). This will take effect provided the fixedMobCapacity is not exceeded
**needSkillForFly - [Boolean] Can fly without soaring skill or not
**timeOut - Time in milliseconds to warp player out and starts counting upon player enters map
**timeLimit - Time in seconds to warp player out. (Used to show at Clock)
**lvLimit - Minimum level to enter the map
**lvForceMove - Maximum level which can stay in the map. Starts counting from THIS value. If lvForceMove is 51, means if level is equals or more than 51, then warp out.
**damageCheckFree - Ignore checking same amount of damages received by player (Used in fishing maps)
**consumeItemCoolTime - Consume use items delay. In seconds
**everlast - Used in Guild PQ and Happyville maps. (Items will not disappear when is dropped inside the map until a player leaves the map)
**link - Since Nexon created many maps of the same type, this <link> field will be the main/first map. (Eg: Mulung PQs)


====Back - Contains information relating to how background image(s) are positioned on the map.====
The 'back' section of a map consists of various 'links' that help build up the background in that specific map. These links contain no images; just information and values that tells the client to fetch specific data from the 'Back.img' (where all background images are stored) property and where or how to place them in each map.

Each 'back' section of a map has several numbered properties in it once opened. These properties always start from 0 and go up numerically. Depending on which number the background image is set on, it can either be infront of another background image, or behind it.

Example 1: A background image set in the property of '0', will '''always''' be behind '''every''' other background image.

Example 2: A background image set in the property of '14' will '''always '''be behind a background image set in the property of 15,''' and''' any other background image set on a property above 15.

Example 3: A background image set in the property of '10' will '''always''' be infront of a background image set in the property of 9, '''and''' any other background image set on a property below 9.

Thus, judging from the examples above; it can be said that the '''lower''' the property number set on a background image is, the further '''behind''' it will be. The '''higher''' the property number set on a background image is, the more infront of '''other''' background images it will be. The only exception being when the 'front' value is set to 1, in which that specific background image will be infront of '''everything''' on the map, including the player.

Moving on, each property has information and values in it, that determine how the background image will appear in the map. Namely, these include:
*a - Normally set to the value of 255.
*ani - Stands for 'animation' or 'animated'. This links to the 'ani' property of the available background sets in the 'Back.img' tree. Images here are dynamic; meaning that they move, hence the term 'animation'. If a value of 1 is set on 'ani' under the 'back' property of a map, it means that the client will fetch background images from the 'ani' section under the 'Back.img' tree, from an available background set. An example of a back would be 'timeTemple.img' or 'Amoria.img'.








*life
*reactor
*0
*1
*2
*3
*4
*5
*6
*7
*clock
*pulley
*healer
*monsterCarnival
*snowBall
*weather
*user
*BuffZone
*noSkill
*seat
*battleField
*snowMan
*mobMassacre
*swimArea
*nodeInfo
*shipObj
*coconut
*ToolTip
*miniMap
*[[footholds|foothold]]
*ladderRope
*[[portal]]

A player jumping in MapleStory.

Movement packets are used to update player and mob (monster) movements on the map. There are multiple packets with varying structure that accomplish this. Each of these packets contain the same structure for "movement fragments" within them. These movement fragments illustrate what the player/mob was doing during that time segment. Each movement packet illustrates what the player/mob was doing over a period of a half a second. The server is tasked with reading these packets and distributing the movement information to clients whom are on the same map (also must be on the same channel and map).

Player movement

Each client is tasked with sending packet updates to the server for the player it manages. The movement packet is parsed by the server and reconstructed into another (similar) packet containing the movement fragments from the original packet and is then sent to other clients on the same map. These movement fragments are then played pack as "ghosts" so to speak to visualize other players on the same map.

Mob movement

Mob movement is very similar to player movement, however additional information about the mob are also included in each packet. Only clients that control the monster are allowed to transmit movement updates for the monster. Clients are asked by the server to move various monsters in advance. Once tasked with monster movement that client is under control of the monsters every move. The controlling client for each monster sends movement packets for each monster it controls. Each monster movement received by the server and then distributed to all the players whom are on the same map.

Movement fragments

There are 5 types of movement fragments. Each of these fragments have a different internal structure and purpose. The 5 movement fragment types are:

Absolute Movement
Jump Down Movement
Relative Movement
Instant Movement
Equipment Movement

Absolute movement is the most commonly used movement fragment as it is used for basic movement such as walking along a foothold and moving the air. Relative movement is another commonly used fragment as it shows the entities jumping. Instant movement is used for teleportation and skills that move the character.

Fragment optimization

To reduce the amount of data transferred to and from the server, optimization is performed by the client to eliminate repeated/unnecessary packets. For example if the entity is not moving, movement fragments showing the players same un-moved location are removed. If the entity did not move since the last movement packet was sent, no movement packet will be sent. Additionally, fragments that can be reconstructed using the previous fragments velocity are left out. Moving in a straight line is a good example of this, where the player does not change speed. Typically a single packet will be sent to mark the players starting position with a duration that stretches the entire 500 milliseconds. Jumping is another occasion that optimization is performed as various fragments can be implied given the previous segments position and velocity. Instead of using absolute movement fragments to illustrate every frame of the jump, only a couple fragments are used to illustrate the same effect.

Genuine client verification

Part of the packet is allocated to verify the integrity of the client. For example, the first 5 bytes and the last 18 bytes of the player movement packet are used to verify integrity of the client. This is done by generating bytes that have no noticeable pattern to the outside observer, however both the client and server understand the pattern and agree with the pattern it creates/receives. Most private servers do not check this operation. Private servers typically skip over this section and assume that the data is valid.

Timing delay

Since each movement update is sent out every 500 milliseconds, there is a hard minimum of a 500 millisecond delay (lag) for other clients to receive this information. Transfer rates introduce additional delays on top of the 500 millisecond delay. Transfer rate between originating client and the server introduces delay, computational time on the server creates additional delay and finally transfer rate between the server and the originating client introduces additional delay. Clients are built to handle this properly to a maximum extent of delay. When significant delay causes packets to miss deadlines causing movement replay to run out, the client estimates the most likely action by continuation of movement. However, at a certain point the client no longer attempts to estimate the position of the entity and locks its position until the movement packet is received.

Packet Structure

There are nine different types of packets that involve movement. Eight of the Nine include movement fragments. These are:

Client: Move player (0x26)
Client: Move life (0x9D)
Client: Move pet (0x8C)
Client: Move summon (0x94)
Server: Move player (0x8D)
Server: Move monster (0xB2)
Server: Move pet (0x81)
Server: Move summon (0x88)
Server: Move monster response (0xB3)

Move monster response is the only packet on the list that does not include movement fragments. Even though the packet does not contain packet fragments, it is important to keep on the list as it is a response from the server to the client that sent the movement report including additional information for about the monster in the response.

Movement fragmentation section

This section illustrates how to parse the movement fragments out of the section.

MovementFragmentation = [count:uint8][fragment:MovementFragment]*count

The first byte "count" is the amount of movement fragments in the packet. After count count amount of MovementFragments are listed. Movement packets have a structure based on their type. Parsing these movement fragments are listed below.

MovementFragment = [fragment_type:uint8][...fragment data...]

Each MovementFragment starts with a byte that determines which of the five movement types the following movement fragment is. The list shown below shows what values relate to which fragment types:

AbsoluteMovement - 0, 5, 17
RelativeMovement - 1, 2, 6, 12, 13, 16
InstantMovement - 3, 4, 7, 8, 9, 14
EquipMovement - 10
JumpDownMovement - 11

Given the movement fragment type is known, the following method listed bellow will parse the given MovementFragment type.

Absolute movement:
[position_x:int16][position_y:int16][velocity_x:int16][velocity_y:int16][unknown:uint16][state:uint8][duration:uint16]
Relative movement:

On jumps duration is 0

[velocity_x:int16][velocity_y:int16][state:uint8][duration:uint16]
Instant movement:
[position_x:int16][position_y:int16][velocity_x:int16][velocity_y:int16][state:uint8]
Equip movement:
[data:uint8]
Jump down movement:
[position_x:int16][position_y:int16][velocity_x:int16][velocity_y:int16][unknown:uint16][foothold_id:uint16][state:uint8][duration:uint16]
Description of variable names:
position_x and position_y: position of the character at the start of the movement
velocity_x and velocity_y: velocity components of the character at the start of the movement
unknown: Truly unknown. Potentially another source of genuine client verification. Does not seem to make a difference with private servers so far.
state: (Stance) Number representing what the player looks like during the movement fragment. A list of these is written below
duration: The duration of the movement fragment in milliseconds.
foothold_id: The foothold the player is jumping off of.
data: (equip data) unknown
State / Stance

This is a number that represents the stance of the player (what the player looks like). For entries that list "x / y" both numbers listed, x and y, relate to the same stance. The difference between the two is the direction of the stance. Left is odd (Left side), right is even (right side).

3 / 2: Walk
5 / 4: Standing
7 / 6: Jumping & Falling
9 / 8: Normal attack
11 / 10: Prone
13 / 12: Rope
15 / 14: Ladder

An NPC (Non-Player Character) is any character you encounter and interact with in MapleStory that is not controlled by other human players -- they 

A Non-Player Character in MapleStory.

are generated and controlled by the game server. Their purpose is to serve quests or challenges, help or entertain.

NPCs are located almost anywhere in the Maple World, with their name displayed in yellow beneath them, and sometimes even a short description in addition.

Using the left button mouse click, players are able to interact with them to initiate a conversation or a quest. NPCs that serve quests will have a lightbulb icon to indicate they have a quest available.

Dialogs

Everytime you speak with an NPC, you encounter a dialog window. Here's a list of the available dialoges:

OK Dialog - A dialog with text displayed and an OK button to terminate the conversation.
Next Dialog - A dialog with text displayed and a Next button to continue the conversation.
Yes/No Dialog - A dialog with text displayed and a Yes button to proceed or a No button to terminate the conversation.
Selection Dialog - A dialog with text displayed and multiple choice selections available to choose from.
Accept/Decline Dialog - A dialog with text displayed and an Accept button to accept or a No button to decline.
Style Dialog - A dialog with text displayed at top and a style selection area (Hair, Skin or Face).
NPCs' Scripting

In the private server scene, NPCs are executed through scripts. Theoretically speaking, NPCs can be scripted using any capable scripting language as long as the server's coding languing is capable of executing them. In OdinMS for example, NPCs are scripted in JavaScript (.js) and are located in the scripts\npc folder.

Here's an example of a script taken from OdinMS:

function start() {
	cm.sendOk("Hi, I'm Cody! How are you today?");
}

# Physics 
File:Placeholder

The physics in MapleStory are purely geometric and involve no pixel checking.

How to handle physics

The physics behind player and mob jumps/falls are based off traditional kinematic equations. Values for all measurements can be found in Map.wz/Physics.img/*. All units are in pixels/seconds. For example, gravity is listed as "2000" under "Map.wz/Physics.img/gravityAcc". The units for which (acceleration in this case: length / time^2) would be px/s^2 (pixels per second squared). Walk speed given in "Map.wz/Physics.img/walkSpeed" is listed at "125" corresponding to 125 px/s (pixels per second). The walk speed provided is the maximum speed achievable with traditional movement for characters with no additional speed improvements by skills or items. Keep in mind that a powerful knockback from a boss skill is an example where a character can move faster than walk speed. Speed improvements are simply added on to the base speed.

Forces and frictional forces are also at play to increase/decrease player velocity to reach the given target velocity. Given a stationary character, pressing right or left will not get the player to immediately reach the maximum velocity. Likewise lifting off the movement key will not cause the character to stop immediately. Instead, forces and friction are used to incrementally increase/decrease the speed to obtain a realistic change in velocity. The forces on snow-less land are strong enough that many players do not notice. Locations where player acceleration are shown more prominently are locations such as El-Nath (snowy regions). In snowy regions, acceleration components are easily seen with sluggish responses and sliding changes in comparison to on typical land.

Physics.img

This section shows various entries located in "Map.wz/Physics.img/". Each entry is listed with the associated name, value, applicable units and optionally a note further describing its usage.

fallSpeed = 670 px/s (Maximum speed falling in the downwards direction)

gravityAcc = 2000 px/s^2

jumpSpeed = 555 px/s (velocity component along the y-axis)

swimSpeed = 140 px/s

walkSpeed = 125 px/s (Base speed of a character)

# Portals

A standard type 2 portal

Portals are used in maplestory to travel from map to map, to teleport across the map, and to initiate scripts.

Portals in the WZ Data
Portal data
pt
The type of the portal (see list below).
pn
The name of the portal.
Other portals points to a portal by the name of the portal.
tm
The id of the map that the portal warps to.
tn
The name of the portal (which is in the target map) to warp to.
x
The X position of the portal
y
The Y position of the portal
horizontalImpact
The horizontal force applied from spring portals (such as those used mainly in jump quests)
verticalImpact
The vertical force
script
The script called upon activation of the portal
onlyOnce
The portal can only be used once and then it deactivates
hideTooltip
Hides the name of the next map that appears when you stand at a portal
delay
The delay after the portal is activated before it takes effect.
Types of portals
0
Name: sp
sp
Full Name: Start Point
Visible: No
Warps: No
Automatic: No
Script: No
Impact: No
Description: The point where the player starts in
1
Name: pi
pi
Full Name: Portal Invisible
Visible: No
Warps: Yes
Automatic: No
Script: No
Impact: No
Description: An invisible portal that can warp
2
Name: pv
pv
Full Name: Portal Visible
Visible: Yes
Warps: Yes
Automatic: No
Script: No
Impact: No
Description: A normal portal
3
Name: pc
pc
Full Name: Portal Collision
Visible: Yes
Warps: Yes
Automatic: Yes
Script: No
Impact: No
Description: A portal that invokes whenever it has a collision with the player
4
Name: pg
pg
Full Name: Portal Changable
Description: When a portal points to it, warps to a sp. We need more info on this.
5
Name: pgi
pgi
Full Name: Portal Changable Invisible
Description: When a portal points to it, warps to a sp. Also is invisible. We need more info on this as well.
6
tp
Name: tp
Full Name: Town Portal Point
Description: 'Town Portal' used for the skill Mystic Door. Same door is used as animation at other end of the portal, too.
7
ps
Name: ps
Full Name: Portal Script
Visible: Unknown
Warps: No
Automatic: No
Script: Yes
Impact: No
Description: A portal that executes a script when a player enters it
8
Name: psi
psi
Full Name: Portal Script Invisible
Visible: No
Warps: No
Automatic: No
Script: Yes
Impact: No
Description: An invisible portal that executes a script when a player enters it
9
Name: pcs
pcs
Full Name: Portal Collision Script
Visible: No
Warps: No
Automatic: Yes
Script: Yes
Impact: No
Description: A portal that executes a script whenever it has a collision with the player
10
Name: ph
ph
Full Name: Portal Hidden
Visible: Yes
Warps: Unknown
Automatic: No
Script: No
Impact: No
Description: A portal that leaves a clue after it. We need more info on this.
11
psh 5
psh 4
psh 3
psh 2
psh 1
Name: psh
psh default
Full Name: Portal Script Hidden
Visible: Yes
Warps: No
Automatic: No
Script: Yes
Impact: No
Description: A portal that executes a script when a player enters it and leaves a clue
12
Name: pcj
pcj
Full Name: Portal Collision Jump
Visible: No
Warps: No
Automatic: Yes
Script: No
Impact: Yes
Description: Applies a vertical boost to the player
13
pci
Name: pci
Full Name: Portal Collision Custom
Visible: No
Warps: No
Automatic: Yes
Script: No
Impact: Yes
Description: Applies a boost in the specified x and y directions
14
Name: pcig
pcig
Full Name: Portal Collision Changeable?
Description: Unknown - If anyone has any info on this, please provide it.

# Tiles

'''Tiles''' make up the ground and sometimes even the walls and ceilings. They are 90x60 images that repeat and fill up a great deal of space. In some areas, particularly Ellinia Forest, they are rarely used

==Tiles in the WZ Data==
*"Map"
**"Map"
***"Map"+mapid[0]
****mapid
*****"info"
******"tS" - Specifies the tile set which all the tiles in this layer use.
*****"tile"
******tileid - Consecutive integers starting at 0
*******"x" - The x coordinate of the tile
*******"y" - The y coordinate of the tile
*******"u" - The type of tile
*******"no" - The specific variant of that tile
**"Tile"
***tS
****u
*****no - The image for the tile
******"z" - The depth of the tile

