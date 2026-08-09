;; #state features
9
+rloc[c]
+armempty[]
+closed[d02]
+in[o1,r2]
-closed[d02]
+rloc[r1]
+rloc[r2]
+holding[o1]
+in[o1,r1]

;; Mutex Groups
9
0 0 +rloc[c]
1 1 +armempty[]
2 2 +closed[d02]
3 3 +in[o1,r2]
4 4 -closed[d02]
5 5 +rloc[r1]
6 6 +rloc[r2]
7 7 +holding[o1]
8 8 +in[o1,r1]

;; further strict Mutex Groups
0

;; further non strict Mutex Groups
0

;; known invariants
0

;; Actions
13
0
-1
-1
-1
1
0 -1
0 5  -1
0 0  -1
1
4 0 -1
0 6  -1
0 0  -1
1
5 -1
0 0  -1
0 5  -1
1
4 6 -1
0 0  -1
0 6  -1
1
2 0 -1
0 4  -1
0 2  -1
1
2 6 -1
0 4  -1
0 2  -1
0
3 6 -1
-1
-1
1
3 6 1 -1
0 7  -1
0 1  0 3  -1
1
8 5 1 -1
0 7  -1
0 1  0 8  -1
0
5 -1
-1
-1
0
7 -1
-1
-1
1
5 7 -1
0 8  0 1  -1
0 7  -1

;; initial state
3 2 1 0 -1

;; goal
8 -1

;; tasks (primitive and abstract)
19
0 __noop
0 move[c,r1,d01]
0 move[c,r2,d02]
0 move[r1,c,d01]
0 move[r2,c,d02]
0 open[c,r2,d02]
0 open[r2,c,d02]
0 __method_precondition_achieve-goals-pickup_split[r2,o1]
0 pickup[o1,r2]
0 pickup[o1,r1]
0 __method_precondition_release-putdown_abstract_split[r1,o1]
0 __method_precondition_release-putdown_abstract_base[o1]
0 putdown[o1,r1]
1 __top[]
1 achieve-goals[]
1 move_abstract[]
1 open_abstract[]
1 pickup_abstract[o1]
1 release[]

;; initial abstract task
13

;; methods
16
__top_method
13
14 -1
-1
finished
14
0 -1
-1
achieve-goals-move
14
15 14 -1
0 1 -1
newMethod24
15
1 -1
-1
newMethod24
15
2 -1
-1
newMethod24
15
3 -1
-1
newMethod24
15
4 -1
-1
achieve-goals-open
14
16 14 -1
0 1 -1
newMethod25
16
5 -1
-1
newMethod25
16
6 -1
-1
<achieve-goals-pickup;achieve-goals-pickup_splitted_1[o1];_splitting_method_achieve-goals-pickup_splitted_1;0;-1,1,2>
14
7 17 18 -1
0 1 0 2 1 2 -1
newMethod22
17
8 -1
-1
newMethod22
17
9 -1
-1
<<release-putdown_abstract;release-putdown_abstract_splitted_2[o1];_splitting_method_release-putdown_abstract_splitted_2;1;0,-1,2,3>;putdown_abstract[];newMethod23;2;0,1,-1,3>
18
11 10 12 14 -1
1 3 1 2 0 1 0 3 0 2 2 3 -1
release-move
18
15 18 -1
0 1 -1
release-open
18
16 18 -1
0 1 -1
