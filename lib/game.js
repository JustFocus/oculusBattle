(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var Game = Oculus.Game = function () {
		this.units = [];
		this.bases = [];
		this.timer = 0;
		this.addBases();
		this.clickCount = 0;
		this.selectedBase = undefined;
	};

	Game.BG_COLOR = "#000000";
	Game.DIM_X = 1000;
	Game.DIM_Y = 600;
	Game.FPS = 32;
	// TODO: Dynamic bases?
	// Game.NUM_BASES = 10;
	Game.BASE_LOCS = [
		{id: 1, xCoord: 100, yCoord: 350, team: 'player', value: 30, startValue: 30},
		{id: 2, xCoord: 900, yCoord: 350, team: 'cpu', value: 30, startValue: 30},
		{id: 3, xCoord: 500, yCoord: 325, team: 'neutral', value: 10, startValue: 10},
		{id: 4, xCoord: 500, yCoord: 225, team: 'neutral', value: 10, startValue: 10},
		{id: 5, xCoord: 500, yCoord: 125, team: 'neutral', value: 10, startValue: 10},
		{id: 6, xCoord: 500, yCoord: 425, team: 'neutral', value: 10, startValue: 10},
		{id: 7, xCoord: 500, yCoord: 525, team: 'neutral', value: 10, startValue: 10},
	];

	Game.prototype.add = function (obj) {
		if (obj instanceof Oculus.Unit) {
			this.units.push(obj);
		} else if (obj instanceof Oculus.Base) {
			this.bases.push(obj);
		} else {
			throw "Attempted to add invalid object to game.";
		}
	};

	Game.prototype.addBases = function () {
		var base;
		for (var i = 0; i < Game.BASE_LOCS.length; i++) {
			base = new Oculus.Base({
				id: i,
				pos: [Game.BASE_LOCS[i].xCoord, Game.BASE_LOCS[i].yCoord],
				radius: (Game.BASE_LOCS[i].value * 2),
				value: Game.BASE_LOCS[i].value,
				team: Game.BASE_LOCS[i].team,
				game: this,
				startValue: Game.BASE_LOCS[i].startValue
			});

			this.add(base);
		}
	};
	// Game.prototype.addUnits = function () {
	//
	// };

	Game.prototype.allObjects = function () {
		return [].concat(this.bases, this.units);
	};

	Game.prototype.checkCollisions = function () {
		var game = this;
		this.units.forEach(function(unit){
			game.bases.forEach(function(base){
				if (unit.target.id === base.id){
					if (unit.hasCollided(base)) {
						game.remove(unit);
						game.handleAttack(base, unit);
					}
				}
			});
		});
	};

	Game.prototype.draw = function (context) {
    context.clearRect(0, 0, Game.DIM_X, Game.DIM_Y);
    context.fillStyle = Game.BG_COLOR;
    context.fillRect(0, 0, Game.DIM_X, Game.DIM_Y);

    this.allObjects().forEach(function (object) {
      object.draw(context);
    });
  };

	Game.prototype.moveUnits = function (delta) {
		this.units.forEach(function (object) {
			object.move(delta);
		});
	};

	Game.prototype.remove = function (obj) {
		if (obj instanceof Oculus.Unit) {
			this.units.splice(this.units.indexOf(obj), 1);
		} else {
			throw "Attempted to remove invalid object from game.";
		}
	};

	Game.prototype.step = function (delta) {
		this.timer += delta;
		this.moveUnits(delta);
		this.checkCollisions();
		// TODO: increase unit count based on delta and timer
		this.increaseUnitCount();
		if (this.isGameOver()) {alert( "game over!");}
		// TODO: do something when game is over.
	};

	Game.prototype.isGameOver = function () {
		var baseCounts = [0, 0];
		for (var i = 0; i < this.bases.length; i++) {
			if ( this.bases[i].team === 'player') {
				baseCounts[0] += 1;
			} else if (this.bases[i].team === 'cpu') {
				baseCounts[1] += 1;
			}
		}
	  return (baseCounts[1] === 0 || baseCounts[0] === 0);
	};

	Game.prototype.increaseUnitCount = function () {
		for (var i = 0; i < this.bases.length; i++) {
			if (this.bases[i].team != 'neutral') {
				this.bases[i].value += this.bases[i].startValue / 1000;
			}
		}
	};

	Game.prototype.handleClick = function (event, offLeft, offRight) {
		this.clickCount += 1;
		var clickedBase = this.parseClick(event.pageX - offLeft, event.pageY - offRight);
		console.log(clickedBase);
		if (typeof clickedBase === 'undefined'){
			this.clickCount = 0;
		} else if ( this.clickCount === 1 ) {
			if (clickedBase.team === 'neutral') {
				this.selectedBase = undefined;
				this.clickCount = 0;
			} else {
			this.selectedBase = clickedBase;}
		} else if ( this.clickCount === 2 ) {
			if (clickedBase.id === this.selectedBase.id) {
				this.clickCount = 0;
				this.selectedBase = undefined;
			} else {
				this.selectedBase.spawnUnits(clickedBase);
				this.selectedBase = undefined;
				this.clickCount =0;
			}
		}
	};

	Game.prototype.parseClick = function (xClick, yClick) {
		for (var i = 0; i < this.bases.length; i++) {
			if( Oculus.Utils.dist([xClick, yClick], this.bases[i].pos)
			< this.bases[i].radius ) {return this.bases[i];}
		}
	};

	Game.prototype.handleAttack = function (base, unit) {
		if (base.team === unit.team) {
			base.value += unit.value;
		} else {
			base.value -= unit.value;
		}

		if (base.value <= 0) {
			base.team = unit.team;
			base.value = 1;
		}

	};

})();
