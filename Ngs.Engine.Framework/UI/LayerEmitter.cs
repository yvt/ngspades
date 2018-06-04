//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.Linq;
using Ngs.Engine.Native;
using Ngs.Engine.Presentation;
using Ngs.Interop;

namespace Ngs.UI {
    /// <summary>
    /// Maintains a layer tree constructed from a series of commands describing the layer tree.
    /// Implements the layer reconciliation algorithm.
    /// </summary>
    /// <seealso cref="View.RenderContext" />
    internal sealed class LayerEmitter {
        sealed class Node {
            readonly LayerEmitter emitter;

            public Node(LayerEmitter emitter, Node parent) {
                this.emitter = emitter;
                this.parent = parent;
            }

            IUnknown pfNode;
            INgsPFNodeGroup pfGroup;
            INgsPFLayer pfLayer;

            IUnknown pfChildSetNode;
            IUnknown pfMaskNode;

            readonly Node parent;

            /// <summary>
            /// The layer info used to create this node from.
            /// </summary>
            LayerInfo info;

            // These are the set of child nodes supplied by the producer.
            List<Node> ordinalChildDefs;
            Dictionary<object, Node> keywordChildDefs;

            Node mask;

            // These are the ordered list of child nodes to be outputted to this node.
            List<IUnknown> children = new List<IUnknown>();
            List<IUnknown> newChildren = new List<IUnknown>();
            List<object> newChildrenRaw = new List<object>();

            /// <summary>
            /// An index into <see cref="ordinalChildDefs" />.
            /// </summary>
            int nextIndex;

            /// <summary>
            /// This flag is used when is this node is contained by a parent node's
            /// <see cref="keywordChildDefs" /> or <see cref="mask" />.
            /// Indicates whether this node was touched during the current update.
            /// </summary>
            bool visited;

            public void Enter(LayerInfo info) {
                if (info != null) {
                    if (this.info != null && this.info.GetType() != info.GetType()) {
                        // The dynamic type has changed - invalidate the layer
                        this.pfLayer = null;
                        this.info = null;
                    }

                    if (this.pfLayer == null) {
                        this.pfLayer = emitter.context.CreateLayer();
                    }

                    info.UpdateLayer(this.pfLayer, this.info);
                } else {
                    // Creation of the corresponding node is deferred until the call to `Leave`.
                    this.pfLayer = null;
                }
                this.info = info;

                this.newChildrenRaw.Clear();

                // Clear all "visited" flags
                this.nextIndex = 0;
                if (keywordChildDefs != null) {
                    foreach (var child in keywordChildDefs) {
                        child.Value.visited = false;
                    }
                }
                if (this.mask != null) {
                    this.mask.visited = false;
                }
            }

            public Node Leave() {
                // Process `newChildrenRaw` and extract native node objects.
                this.newChildren.Clear();
                this.newChildren.Capacity = this.newChildrenRaw.Capacity;
                foreach (var item in this.newChildrenRaw) {
                    switch (item) {
                        case Node node:
                            if (node.pfNode != null) {
                                this.newChildren.Add(node.pfNode);
                            }
                            break;
                        case IUnknown nativeNode:
                            this.newChildren.Add(nativeNode);
                            break;
                        default:
                            throw new InvalidOperationException();
                    }
                }

                // Remove stale child nodes
                if (this.ordinalChildDefs != null) {
                    this.ordinalChildDefs.RemoveRange(this.nextIndex, this.ordinalChildDefs.Count - this.nextIndex);
                }
                if (this.keywordChildDefs != null) {
                    var removedKeys = from e in this.keywordChildDefs where !e.Value.visited select e.Key;
                    // Ugh, ugly dynamic allocations...
                    foreach (var key in removedKeys.ToList()) {
                        this.keywordChildDefs.Remove(key);
                    }
                }
                if (this.mask != null && !this.mask.visited) {
                    this.mask = null;
                }

                // Decide what kind of PF node we should be created for this node.
                IUnknown pfNodeSet;
                if (newChildren.Count == 0) {
                    this.pfGroup = null;
                    pfNodeSet = null;
                } else if (newChildren.Count == 1) {
                    this.pfGroup = null;
                    pfNodeSet = newChildren[0];
                } else {
                    // Try not to recreate a node group
                    if (children.Count != newChildren.Count || this.pfGroup == null) {
                        goto notEqual;
                    }

                    for (int i = 0, count = children.Count; i < count; ++i) {
                        if (children[i] != newChildren[i]) {
                            goto notEqual;
                        }
                    }

                    goto done;
                notEqual:
                    this.pfGroup = emitter.context.CreateNodeGroup();
                    foreach (var child in newChildren) {
                        this.pfGroup.Insert(child);
                    }

                done:
                    pfNodeSet = this.pfGroup;
                }

                // Swap `children` and `newChildren`
                var t = children;
                children = newChildren;
                newChildren = t;
                t.Clear();

                if (this.pfLayer == null) {
                    this.pfNode = pfNodeSet;
                } else {
                    if (pfNodeSet != pfChildSetNode) {
                        this.pfLayer.Child = pfNodeSet;
                        pfChildSetNode = pfNodeSet;
                    }

                    // Make sure to set the layer mask
                    var pfMaskNode = this.mask?.pfNode;
                    if (pfMaskNode != this.pfMaskNode) {
                        this.pfMaskNode = pfMaskNode;
                        this.pfLayer.Mask = pfMaskNode;
                    }

                    this.pfNode = this.pfLayer;
                }

                return parent;
            }

            public Node DefineChild(object key) {
                Node child;

                if (key == null) {
                    if (ordinalChildDefs == null) {
                        ordinalChildDefs = new List<Node>();
                    }
                    if (nextIndex >= ordinalChildDefs.Count) {
                        ordinalChildDefs.Add(new Node(emitter, this));
                    }
                    child = ordinalChildDefs[nextIndex++];
                } else {
                    if (keywordChildDefs == null) {
                        keywordChildDefs = new Dictionary<object, Node>();
                    }
                    if (keywordChildDefs.TryGetValue(key, out child)) {
                        if (child.visited) {
                            throw new ArgumentException("The specified key already exists in the current node.", nameof(key));
                        }
                        child.visited = true;
                    } else {
                        child = new Node(emitter, this)
                        {
                            visited = true
                        };
                        keywordChildDefs.Add(key, child);
                    }
                }

                // `child.pfNode` is not final until `child.Leave()` is called. This also explains
                // why we have `Node.newChildrenRaw`.
                newChildrenRaw.Add(child);

                return child;
            }

            public Node DefineMask() {
                if (this.pfLayer == null) {
                    throw new InvalidOperationException("The current node is not a layer.");
                }
                if (this.mask == null) {
                    this.mask = new Node(emitter, this);
                }
                if (this.mask.visited) {
                    throw new InvalidOperationException("The current layer already has a mask group.");
                }
                this.mask.visited = true;
                return this.mask;
            }

            public void DefineInstantiatedLayer(IUnknown layer) {
                this.newChildrenRaw.Add(layer);
            }

            public IUnknown PresentationNode { get => pfNode; }
        }

        readonly INgsPFContext context;

        readonly Node root;
        Node current;

        public LayerEmitter(INgsPFContext context) {
            this.context = context;
            this.root = new Node(this, null);
        }

        public IUnknown Root { get => root.PresentationNode; }

        public void BeginUpdate() {
            current = root;
            root.Enter(null);
        }

        public void EndUpdate() {
            // FIXME: Report mismatched `BeginLayer`/`EndLayer`?
            while (current != root) {
                current = current.Leave();
            }
            current = null;

            root.Leave();
        }

        /// <summary>
        /// Starts encoding a layer or node group.
        /// </summary>
        /// <param name="key">The key used to identify the new node within the parent.
        /// Can be <c>null</c>.</param>
        /// <param name="info">The layer info or <c>null</c>.</param>
        public void BeginLayer(object key, LayerInfo info) {
            var child = current.DefineChild(key);
            child.Enter(info);
            current = child;
        }

        /// <summary>
        /// Starts encoding a mask group.
        /// </summary>
        public void BeginMaskGroup() {
            var child = current.DefineMask();
            child.Enter(null);
            current = child;
        }

        /// <summary>
        /// Ends encoding a layer, node group, or a mask group.
        /// </summary>
        public void End() {
            if (current == root) {
                throw new InvalidOperationException("The current position is already at the root level.");
            }
            current = current.Leave();
        }

        public void EmitInstantiatedLayer(IUnknown layer) {
            current.DefineInstantiatedLayer(layer);
        }
    }
}
