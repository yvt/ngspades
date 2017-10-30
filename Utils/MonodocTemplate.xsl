<?xml version="1.0"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
  <xsl:output
    encoding="UTF-8"
    indent="yes"
    method="xml"
    omit-xml-declaration="yes"
  />

  <xsl:template match="Page">
    <html>
      <head>
        <title>
          <xsl:value-of select="Title" />
        </title>
        <meta http-equiv="Content-Type" content="text/html; charset=UTF-8" />
        <xsl:call-template name="create-default-style" />
        <xsl:call-template name="create-default-script" />
      </head>
      <body>
        <!-- HEADER -->
        <xsl:call-template name="create-default-collection-title" />
        <xsl:call-template name="create-index" />
        <xsl:call-template name="create-default-title" />
        <xsl:call-template name="create-default-summary" />
        <xsl:call-template name="create-default-signature" />
        <xsl:call-template name="create-default-remarks" />
        <xsl:call-template name="create-default-members" />
        <hr size="1" />
        <xsl:call-template name="create-default-copyright" />
      </body>
    </html>
  </xsl:template>

  <!-- IDENTITY TRANSFORMATION -->
  <xsl:template match="@*|node()">
    <xsl:copy>
      <xsl:apply-templates select="@*|node()" />
    </xsl:copy>
  </xsl:template>

  <xsl:template name="create-default-style">
    <style>
      a { text-decoration: none }

      body, html {
        font-family: '.SFNSText-Light', sans-serif;
        background: white;
        color: #101010;
        line-height: 1.5;
        letter-spacing: -0.3;
      }
      body {
        margin: 20px;
      }

      a {
        text-decoration: none;
        color: rgba(50, 100, 200, 1);
        border-bottom: 1px solid rgba(50, 100, 200, 0.2);
        transition: 200ms border-color;
      }
      a:hover {
        border-bottom: 1px solid rgba(50, 100, 200, 0.9);
      }

      div.SideBar {
        padding-left: 1em;
        padding-right: 1em;
        right: 0;
        float: right;
        border: thin solid black;
        background-color: #f2f2f2;
      }

      .CollectionTitle { font-weight: bold }
      .PageTitle {
        font-size: 200%; font-weight: 200;
        font-family: '.SFNSDisplay-Thin', sans-serif;
        letter-spacing: 0;
        margin: 1em 0;
      }

      .Summary { }
      .Signature { }
      .Remarks { }
      .Members { }
      .Copyright { }

      h2, .Section, h3 {
        font-size: 125%; font-weight: 200;
        margin: 1em 0;
        text-transform: uppercase;
        font-family: '.SFNSDisplay-Light', sans-serif;
        color: #505050;
        letter-spacing: 0;
      }
      h3 {
        text-transform: none !important;
      }
      p.Summary {
        margin-left: 1em;
      }
      .SectionBox { margin-left: 2em }
      .NamespaceName { font-size: 105%; font-weight: bold }
      .NamespaceSumary { }
      .MemberName { font-size: 115%; font-weight: bold; margin-top: 1em }
      .Subsection {
        font-size: 105%; font-weight: 200;
        margin: 1em 0;
        text-transform: uppercase;
        font-style: italic;
        font-family: '.SFNSDisplay-Light', sans-serif;
        color: #505050;
        letter-spacing: 0;
      }
      .SubsectionBox {
        margin-left: 1em; margin-bottom: 1em;
        padding-left: 1em;
        border-left: 4px solid #f0f0f0;
      }

      .CodeExampleTable { background-color: #f5f5dd; border: thin solid black; padding: .25em; }

      .TypesListing {
        border-collapse: collapse;
      }

      td {
        vertical-align: top;
      }
      th {
        text-align: left;
      }

      .TypesListing td {
        margin: 0.;
        padding: .5em;
        border: none;
      }

      .TypesListing th {
        margin: 0px;
        padding: .5em;
        color: #888;
        font-weight: 200;
        font-size: 80%;
        text-transform: uppercase;
        border-bottom: solid #f0f0f0 1px;
      }

      div.Footer {
        border-top: 1px solid gray;
        margin-top: 1.5em;
        padding-top: 0.6em;
        text-align: center;
        color: gray;
      }

      span.NotEntered /* Documentation for this section has not yet been entered */ {
        color: #d06060;
        font-weight: bold;
        font-size: 60%;
        text-transform: uppercase;
      }

      div.Header {
        background: #B0C4DE;
        border: double;
        border-color: white;
        border-width: 7px;
        padding: 0.5em;
      }

      div.Header * {
        font-size: smaller;
      }

      div.Note {
      }

      i.ParamRef {
      }

      i.subtitle {
      }

      ul.TypeMembersIndex {
        text-align: left;
        background: #F8F8F8;
      }

      ul.TypeMembersIndex li {
        display: inline;
        margin:  0.5em;
      }

      table.HeaderTable {
      }

      table.SignatureTable {
      }

      table.Documentation, table.Enumeration, table.TypeDocumentation {
        border-collapse: collapse;
        width: 100%;
      }

      table.Documentation tr th, table.TypeMembers tr th, table.Enumeration tr th, table.TypeDocumentation tr th {
        background: whitesmoke;
        padding: 0.8em;
        border: none;
        text-align: left;
        vertical-align: bottom;
      }

      table.Documentation tr td, table.TypeMembers tr td, table.Enumeration tr td, table.TypeDocumentation tr td {
        padding: 0.5em;
        border: none;
        text-align: left;
        vertical-align: top;
      }

      table.TypeMembers {
        border: none;
        width: 100%;
      }

      table.TypeMembers tr td {
        background: #F8F8F8;
        border: white;
      }

      table.Documentation {
      }

      table.TypeMembers {
      }

      div.CodeExample {
        width: 100%;
        border: 1px solid #DDDDDD;
        background-color: #F8F8F8;
      }

      div.CodeExample p {
        margin: 0.5em;
        border-bottom: 1px solid #DDDDDD;
      }

      div.CodeExample div {
        margin: 0.5em;
      }

      h4 {
        margin-bottom: 0;
      }

      div.Signature {
        border-left: 1px solid #f0f0f0;
        background: #f8f8f8;
        padding: 1em;
        margin: 1em;
        color: #606060;
        font-family: 'Inconsolata', monospace;
      }
    </style>
  </xsl:template>

  <xsl:template name="create-default-script">
    <script type="text/JavaScript">
      function toggle_display (block) {
        var w = document.getElementById (block);
        var t = document.getElementById (block + ":toggle");
        if (w.style.display == "none") {
          w.style.display = "block";
          t.innerHTML = "⊟";
        } else {
          w.style.display = "none";
          t.innerHTML = "⊞";
        }
      }
    </script>
  </xsl:template>

  <xsl:template name="create-index">
    <xsl:if test="
        count(PageTitle/@id) &gt; 0 and
        (count(Signature/@id) &gt; 0 or count(Signature/div/@id) &gt; 0) and
        count(Remarks/@id) &gt; 0 and
        count(Members/@id) &gt; 0
        ">
      <div class="SideBar">
        <p>
          <a>
            <xsl:attribute name="href">
              <xsl:text>#</xsl:text>
              <xsl:value-of select="PageTitle/@id" />
            </xsl:attribute>
            <xsl:text>Overview</xsl:text>
          </a>
        </p>
        <p>
          <a>
            <xsl:attribute name="href">
              <xsl:text>#</xsl:text>
              <xsl:value-of select="Signature/@id" />
              <xsl:value-of select="Signature/div/@id" />
            </xsl:attribute>
            <xsl:text>Signature</xsl:text>
          </a>
        </p>
        <p>
          <a>
            <xsl:attribute name="href">
              <xsl:text>#</xsl:text>
              <xsl:value-of select="Remarks/@id" />
            </xsl:attribute>
            <xsl:text>Remarks</xsl:text>
          </a>
        </p>
        <p>
          <a href="#Members">Members</a>
        </p>
        <p>
          <a>
            <xsl:attribute name="href">
              <xsl:text>#</xsl:text>
              <xsl:value-of select="Members/@id" />
            </xsl:attribute>
            <xsl:text>Member Details</xsl:text>
          </a>
        </p>
      </div>
    </xsl:if>
  </xsl:template>

  <xsl:template name="create-default-collection-title">
    <div class="CollectionTitle">
      <xsl:apply-templates select="CollectionTitle/node()" />
    </div>
  </xsl:template>

  <xsl:template name="create-default-title">
    <h1 class="PageTitle">
      <xsl:if test="count(PageTitle/@id) &gt; 0">
        <xsl:attribute name="id">
          <xsl:value-of select="PageTitle/@id" />
        </xsl:attribute>
        <xsl:apply-templates select="PageTitle/node()" />
      </xsl:if>
      <xsl:if test="count(PageTitle/@id) = 0">
        <xsl:text>Nightingales Framework Documentation</xsl:text>
      </xsl:if>
    </h1>
  </xsl:template>

  <xsl:template name="create-default-summary">
    <p class="Summary">
      <xsl:if test="count(Summary/@id) &gt; 0">
        <xsl:attribute name="id">
          <xsl:value-of select="Summary/@id" />
        </xsl:attribute>
      </xsl:if>
      <xsl:apply-templates select="Summary/node()" />
    </p>
  </xsl:template>

  <xsl:template name="create-default-signature">
    <div>
      <xsl:if test="count(Signature/@id) &gt; 0">
        <xsl:attribute name="id">
          <xsl:value-of select="Signature/@id" />
        </xsl:attribute>
      </xsl:if>
      <xsl:apply-templates select="Signature/node()" />
    </div>
  </xsl:template>

  <xsl:template name="create-default-remarks">
    <div class="Remarks">
      <xsl:if test="count(Remarks/@id) &gt; 0">
        <xsl:attribute name="id">
          <xsl:value-of select="Remarks/@id" />
        </xsl:attribute>
      </xsl:if>
      <xsl:apply-templates select="Remarks/node()" />
    </div>
  </xsl:template>

  <xsl:template name="create-default-members">
    <div class="Members">
      <xsl:if test="count(Members/@id) &gt; 0">
        <xsl:attribute name="id">
          <xsl:value-of select="Members/@id" />
        </xsl:attribute>
      </xsl:if>
      <xsl:apply-templates select="Members/node()" />
    </div>
  </xsl:template>

  <xsl:template name="create-default-copyright">
    <div class="Copyright">
      <xsl:apply-templates select="Copyright/node()" />
    </div>
  </xsl:template>
</xsl:stylesheet>
