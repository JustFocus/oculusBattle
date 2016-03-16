(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var Game = Oculus.Game = function () {
		this.units = [];
		this.bases = [];
		this.timer = 0;
		this.addBases(this.generateBases());
		this.clickCount = 0;
		this.selectedBase = undefined;
		this.aiTimer = 0;
		this.ai = new Oculus.AI();
		this.gameState = 1;
	};

	Game.BG_COLOR = "#000000";
	Game.DIM_X = 850;
	Game.DIM_Y = 500;
	Game.FPS = 32;
	Game.BASE_SIZES = [
		10, 10, 10, 10, 10, 10, 10, 10, 10,
		15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
		20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20,
		25, 25, 25, 25, 25, 25,
		30, 30, 30, 30,
		35, 35,
		40
	];
	Game.BASE_LOCATIONS = [

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

	Game.prototype.addBases = function (bases) {
		var base;
		for (var i = 0; i < bases.length; i++) {
			base = new Oculus.Base({
				id: i,
				pos: [bases[i].xCoord, bases[i].yCoord],
				radius: (bases[i].value * 1.5),
				value: bases[i].value,
				team: bases[i].team,
				game: this,
				startValue: bases[i].startValue
			});

			this.add(base);
		}
	};

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
		if (this.allObjects().length === 0) {
			context.font = "bold 40px arial";
			var contText;
			if (this.ai.power === 1) {
				contText =  "Game Over! Click to restart"
			} else {
				contText =  "The computer grows stronger..."
			}
			context.fillText(contText, 100, 100);
			var lvlText = "Click to begin level " + this.ai.power;
			context.fillText(lvlText, 100, 200);
		} else {
	    this.allObjects().forEach(function (object) {
				var that = this;
				object.draw(context, that);
	    }.bind(this));
		}
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
		if (this.gameState){
			this.aiMove(delta);
			this.moveUnits(delta);
			this.checkCollisions();
			this.increaseUnitCount();
			var winner = this.isGameOver();
			if (winner) {
				this.units = [];
				this.bases = [];
				this.timer = 0;
				this.clickCount = 0;
				this.selectedBase = undefined;
				this.aiTimer = 0;
				this.gameState = 0;
				if (winner === 'player') {
					this.ai.power += 1;
				} else {
					this.ai.power = 1;
				}
			}
		}
	};

	Game.prototype.aiMove = function (delta) {
		this.timer += delta;
		this.aiTimer += delta;
		if (this.aiTimer > (Math.random() * 4000 + 1600)) {
			var aiSelection =  this.ai.selectBase(this.bases);
			var aiAttack = this.ai.attackBase(this.bases, aiSelection);
			this.aiTimer = 0;
			if (aiSelection && aiAttack) {
				aiSelection.spawnUnits(aiAttack);
			}
		}
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
	  return (this.winner(baseCounts));
	};

	Game.prototype.winner = function (baseCounts) {
		if (baseCounts[0] !== 0 && baseCounts[1] !==0) {
			return false;
		} else {
			if (baseCounts[0]) {
				return 'player';
			} else { return 'cpu';}
		}
	};

	Game.prototype.increaseUnitCount = function () {
		for (var i = 0; i < this.bases.length; i++) {
			if (this.bases[i].team !== 'neutral' &&
					this.bases[i].value < this.bases[i].startValue * 2) {
				var levelMult;
				if (this.bases[i].team === 'cpu'){
					levelMult = ((this.ai.power - 1) * 0.25) + 1;
				} else {
					levelMult = 1;
				}
				this.bases[i].value += this.bases[i].startValue / (1000 / levelMult);
			}
		}
	};

	Game.prototype.handleClick = function (event, offLeft, offRight) {
		if (this.gameState === 0) {
			this.gameState = 1;
			this.addBases(this.generateBases());
		} else {
			this.clickCount += 1;
			var clickedBase = this.parseClick(event.pageX - offLeft, event.pageY - offRight);
			if (typeof clickedBase === 'undefined'){
				this.clickCount = 0;
				this.selectedBase = undefined;
			} else if ( this.clickCount === 1 ) {
				if (clickedBase.team !== 'player') {
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
					this.clickCount = 0;
				}
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

	Game.prototype.generateBases = function () {

		var bases = [
			{id: 1, xCoord: 75, yCoord: 200, team: 'player', value: 30, startValue: 30},
			{id: 2, xCoord: 725, yCoord: 200, team: 'cpu', value: 30, startValue: 30}
		];
		var baseCount;
		var verticalSplit;
		var padding = 10;
		var maxSize = 50;
		var minPlace;
		var maxPlace;
		var yVal;
		var size;
		var xVal;
		for (var i = 0; i < 2; i++) {
			baseCount = Math.floor(Math.random() * 3) + 1;
			verticalSplit = Math.floor(Game.DIM_Y / baseCount);
			xVal = 50 + ((i + 1) * 175);
			for (var baseNum = 0; baseNum < baseCount; baseNum++) {
				size = Game.BASE_SIZES[Math.floor(Math.random()*Game.BASE_SIZES.length)];
				minPlace = baseNum * verticalSplit + padding + maxSize;
				maxPlace = (baseNum + 1) * verticalSplit - padding - maxSize;
				yVal = Math.floor(Math.random()*(maxPlace-minPlace+1)+minPlace);
				bases.push({
					id: bases.length,
					xCoord: xVal,
					yCoord: yVal,
					team: 'neutral',
					value: size,
					startValue: size
				});
				if (i === 0 ) {
					bases.push({
						id: bases.length,
						xCoord: xVal + 350,
						yCoord: yVal,
						team: 'neutral',
						value: size,
						startValue: size
					});
				}
			}
		}
		return bases;
	};

})();
