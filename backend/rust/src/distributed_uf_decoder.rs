//! # Distributed UnionFind Decoder
//!
//! ## Introduction
//!
//! UnionFind decoder has good accuracy and computational complexity when running on CPU, which is in worst case $O(n α(n))$.
//! In a running quantum computer, the number of errors $n$ that are actively concerned in every round is $O(d^3)$, given the code distance $d$.
//! Suppose every fault tolerant operation requires O(d) rounds, that means we need to solve $n = O(d^3)$ errors in O(d) time.
//! This latency requirement is much stricter than the currently sequential implementation of UnionFind decoder, which is about $O(d^3)$ over the requirement of $O(d)$.
//!
//! We need to design a distributed UnionFind decoder to fit into the timing constraint.
//! This means we need to solve $O(d^3)$ errors in as much close to O(d) time as possible.
//! In this work, we propose a $O(d \log{d})$ average time distributed UnionFind decoder implemented on FPGA(s).
//!
//! ## Background
//!
//! ### Union Find Decoder
//! UnionFind decoder for topological quantum error correction (TQEC) codes is one of the currently most practical decoders both in accuracy and time complexity.
//! It requires at most $O(d)$ iterations, in each iteration the exploratory region of each odd cluster grows.
//! This growing cluster requires a tracking of the disjoint sets, which is extremely efficient using Union Find algorithm.
//! After analyzing each steps in the sequential UnionFind decoder, we found that the Union Find solver is the main challenge that blocks a low-latency distributed version.
//!
//! ### Parallel Union Find Solver
//! There exists some works for parallel Union-Find algorithm, e.g. [arXiv:2003.02351](https://arxiv.org/pdf/2003.02351.pdf),
//!     [arXiv:1710.02260](https://arxiv.org/pdf/1710.02260.pdf).
//! But none of them applies to our requirement direction, which is nano-second level latency with at least $O(d^2)$ concurrent requests needed.
//!
//! ## Design
//!
//! Instead of seeking for a general distributed Union Find algorithm, we try to improve the Union Find performance by exploiting the attributes of TQEC codes.
//! The main property is that, the interactions of the stabilizers are local, meaning that two stabilizers have direct connection only if they're neighbors in the space.
//! Thus, the disjoint set during the execution of UF decoder has an attribute that it's spreading in the space, which has a longest spreading path of length $d$.
//!
//! A naive design would be spreading the root of the disjoint set in the graph.
//! When a union operation should apply to two disjoint sets, the root is updated to the smallest root.
//! This is not considered optimal in sequential union-find algorithms, actually they use rank-based or weight-based merging to improve performance.
//! In our case, however, since the root must be spread to all nodes, which takes O(d) worst case bound, a fixed rule of root selection
//!      (so that node can choose the updated root without querying the root's internal state) is more important than reducing the number of updated nodes.
//! This operation is totally distributed, as merging union will ultimately be updated to the smallest root, although some intermediate state has invalid root.
//!
//! The naive design has a strict $O(d)$ worst case bound for each iteration, and the number of iteration is strictly $d$.
//! Thus, the total complexity is $O(d^2)$, which is growing faster than the time budget of $O(d)$.
//! To solve this gap, we propose a optimized version of distributed UF decoder that still has $O(d^2)$ worst case bound but the average complexity reduces to $O(d\log{d})$.
//!
//! The optimization originates from the key observation that the time is spending on spreading the updated root from one side to the very far end.
//! If we can send the updated root directly from one side to all other nodes, then the problem solves in $O(1)$ strict time bound.
//! But this is problematic in that it requires a complete connection between every two nodes, introducing $O(d^6)$ connections which is not scalable in hardware.
//! To balance between hardware complexity and time complexity, we try to add connections more cleverly.
//! We add connections to a pair of nodes if they're at exact distance of 2, 4, 8, 16, ··· in one dimension and also must be identical in the other dimensions.
//! For example, in a 2D arranged nodes (figure below), the <span style="color: red;">red</span> node connects to the <span style="color: blue;">blue</span> nodes.
//! Every node connects to $O(\log{d})$ other nodes in the optimized design, instead of $O(1)$ in the naive design.
//! This overhead is scalable with all practical code distances, and this will reduce the longest path from $O(d)$ to $O(\log{d})$.
//! We call this a "fast channel architecture".
//!
//! <div style="width: 100%; display: flex; justify-content: center;"><svg id="distributed_uf_decoder_connections_2D_demo" style="width: 300px;" viewBox="0 0 100 100"></svg></div>
//! <script>function draw_distributed_uf_decoder_connections_2D_demo(){let t=document.getElementById("distributed_uf_decoder_connections_2D_demo");if(!t)return;const e=parseInt(10.5);function r(t){for(;1!=t;){if(t%2!=0)return!1;t/=2}return!0}for(let i=0;i<21;++i)for(let n=0;n<20;++n){const o=(n+1.5)*(100/22),c=(i+1)*(100/22);let u=document.createElementNS("http://www.w3.org/2000/svg","circle");u.setAttribute("cx",o),u.setAttribute("cy",c),u.setAttribute("r",100/22*.3),u.setAttribute("fill","rgb(0, 0, 0)"),i==e&&n==e?u.setAttribute("fill","rgb(255, 0, 0)"):(i==e&&r(Math.abs(n-e))||n==e&&r(Math.abs(i-e)))&&u.setAttribute("fill","rgb(0, 0, 255)"),t.appendChild(u)}}document.addEventListener("DOMContentLoaded", draw_distributed_uf_decoder_connections_2D_demo)</script>
//! 
//! The worst case bound of the optimized design seems to be $O(d \log{d})$ at the first glance, but this isn't true when coming to a practical distributed implementation.
//! Considering the format of the messages passing through those connections, it's different from the naive design in that the node cannot easily know
//!     whether the receiver is in the same disjoint set as the sender.
//! It's better to let the receiver to decide whether it should respond to the message, to avoid some inconsistent state sharing.
//! In our design, the message has two field:
//! - the old root of the current node (this old root keeps constant at the beginning of the iteration, updated only at the end of the iteration)
//! - the updated root of the current node
//!
//! When the remote node receives a message, it will drop the message if his old root doesn't match with the old root in the message.
//! This means that they must be in the same disjoint set at the beginning of the iteration, otherwise they won't have a same old root.
//! The worst case comes when there are $O(d)$ equally spaced disjoint sets on a single line (e.g. all the stabilizers on a single line has error syndrome), like below
//!
//! <div style="width: 100%; display: flex; justify-content: center;"><svg id="distributed_uf_decoder_connections_worst_case" style="width: 400px;" viewBox="0 0 400 110"></svg></div>
//! <script>function draw_distributed_uf_decoder_connections_worst_case(){let t=document.getElementById("distributed_uf_decoder_connections_worst_case");if(!t)return;parseInt(10.5);const e="http://www.w3.org/2000/svg",r=["rgb(255, 0, 0)","rgb(0, 0, 255)","rgb(0, 255, 0)","rgb(255, 0, 255)","rgb(0, 255, 255)","rgb(255, 255, 0)"];for(let i=0;i<5;++i)for(let s=0;s<20;++s){const n=(s+1.5)*(400/22),d=(i+1)*(400/22);let o=document.createElementNS(e,"circle");o.setAttribute("cx",n),o.setAttribute("cy",d),o.setAttribute("r",400/22*.3),o.setAttribute("fill","rgb(0, 0, 0)"),2==i&&0!=s&&19!=s?o.setAttribute("fill",r[parseInt((s-1)/3)]):1!=i&&3!=i||0==s||19==s||(s+1)%3!=0||o.setAttribute("fill",r[parseInt((s-1)/3)]),t.appendChild(o)}for(let r=0;r<5;++r){let i=document.createElementNS(e,"rect");i.setAttribute("width",400/22*2),i.setAttribute("height",400/22),i.setAttribute("style","fill:none; stroke:black; stroke-width:3;"),i.setAttribute("x",400/22*(3*r+4)),i.setAttribute("y",400/22*2.5),t.appendChild(i)}}document.addEventListener("DOMContentLoaded",draw_distributed_uf_decoder_connections_worst_case);</script>
//!
//! The root (red block) will have to spread linearly from the left side to the right side.
//! The $O(\log{d})$ direct connections doesn't work in this case because the old root is not the same between disjoint sets (in different colors).
//! After spreading for $O(d)$ time, the system will be below
//!
//! <div style="width: 100%; display: flex; justify-content: center;"><svg id="distributed_uf_decoder_connections_worst_case_after" style="width: 400px;" viewBox="0 0 400 110"></svg></div>
//! <script>function draw_distributed_uf_decoder_connections_worst_case_after(){let t=document.getElementById("distributed_uf_decoder_connections_worst_case_after");if(!t)return;parseInt(10.5);const e="rgb(255, 0, 0)";for(let r=0;r<5;++r)for(let n=0;n<20;++n){const d=(n+1.5)*(400/22),c=(r+1)*(400/22);let i=document.createElementNS("http://www.w3.org/2000/svg","circle");i.setAttribute("cx",d),i.setAttribute("cy",c),i.setAttribute("r",400/22*.3),i.setAttribute("fill","rgb(0, 0, 0)"),2==r&&0!=n&&19!=n?i.setAttribute("fill",e):1!=r&&3!=r||0==n||19==n||(n+1)%3!=0||i.setAttribute("fill",e),t.appendChild(i)}}document.addEventListener("DOMContentLoaded",draw_distributed_uf_decoder_connections_worst_case_after);</script>
//!
//! Thus, the optimized design has only a smaller average time complexity of $O(\log{d})$ per round, but the worst case is still $O(d)$ per round.
//! If the algorithm does not finish in $O(\log{d})$ time bound, we cannot just stop it at the middle because that will cause unrecoverable inconsistent state.
//!
//! The Union-Find operation is solved, but there is still a hard problem, that how to compute the cardinality of a cluster with low complexity?
//! In the sequential union find algorithm, this is done by simply store the state in the root and update on every union operation.
//! Since the union operation happens locally and distributively in our algorithm, it's not easy to decide how to update the cardinality.
//! We solve this problem by maintaining the old cardinality in the root node, and also stores a counter that counts the increment of cardinality in another register.
//! When several disjoint sets merge into a new one, the merged root node will send a direct message to the new root node telling him the old cardinality.
//! The new root will add this cardinality into the counter without changing its old cardinality register.
//! This procedure takes a strict timing bound of $O(\log{d})$ because of the fast channel architecture.
//! In this way, the ultimate root node will receive the old cardinality message from all root nodes that are merged into him for exactly once, which is expected.
//! Note that those intermediate root will also have non-zero counter value, but this doesn't matter, since the old cardinality register keeps constant during iteration.
//!
//! ## Implementation
//!
//! There is a nice attribute of the fast channels in distributed UF decoder, that messages from all the channels can be processed simultaneously.
//! This happens because among those message there is only one that "outstands".
//! The messages can be first filtered simultaneously by the old root field (ignore those whose old root doesn't match to the local register).
//! Then, those that are valid can be compared in a two-two manner, to elect a best one, which takes $O(\log{m})$ circuit level latency in a tree-structured compare.
//! Since the amount of channels for each node is $m = O(\log{d})$, the circuit level latency is just $O(\log{\log{d}})$ which is super scalable.
//! The outstanding message will finally reach the core logic and been executed.
//! Since all the messages are guaranteed to be handled in a single clock cycle, the channel could simply be a FIFO with length 1.
//! This saves a lot of hardware resources.
//!
//! For the cardinality updating message, though, it must be processed sequentially.
//! Thus, it will use another set of fast channels, different from the Union messages discussed above.
//! It could happen that one node receives multiple messages (brokering them to different roots) at the same time, but could not handle them.
//! The channel will be a special one that has a feedback bit, representing whether the last message is already handled.
//! If the sender wants to send a message but the receiver is still busy at handling other messages, it will just delay the message and waiting for next clock cycle.
//! The cardinality update doesn't affect the correctness of union operations, so it has least priority.
//! The iteration stops once there are no messages pending, and till then the cardinality at the new root will increase by the increment counter.
//! This ensures a consistent state at the end of the iteration.
//!
