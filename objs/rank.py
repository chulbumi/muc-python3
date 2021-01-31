# -*- coding: utf-8 -*-
from lib.loader import load_script, save_script
import _pickle

class Rank():
    attr = {}
    path = 'data/config/rank.dat'
    def load(self):
        try:
            with open(self.path) as fp:
                self.attr = load_script(fp)
        except IOError:
            print('%s IOError' % self.path)
            return
        except EOFError:
            print('%s EOFError' % self.path)
            return
        except:
            print('Error %s' % self.path)
            return

    def save(self):
        try:
            with open(self.path, 'w', encoding="utf-8") as fp:
                save_script(fp, self.attr)

        except IOError:
            print('%s IOError' % self.path)
            return
        except EOFError:
            print('%s EOFError' % self.path)
            return
        except:
            print('Error %s' % self.path)
            return

    def write_rank(self, type, name, value, level):
        if type not in self.attr:
            rank = []
            self.attr[type] = rank
        else:
            rank = self.attr[type]
        import copy
        rr = copy.copy(rank)
        for r in rr:
            if r[1] == name:
                rank.remove(r)
                
        if value == -1:
            rank.insert(0, (0, name))
            if len(rank) > 200:
                rank.pop()
            self.save()
            return 1
        else:
            rank.append( (value, name) )
            rank.sort(reverse = True)
            if len(rank) > 200:
                rank.pop()
            self.save()
            return self.read_rank(type, name)
        
    def read_rank(self, type, name):
        if type not in self.attr:
            return 0
        rank = self.attr[type]
        c = 0
        for r in rank:
            c += 1
            if r[1] == name:
                return c
        return 0
        
    def getRankNum(self, type, rank):
        try:
            r = self.attr[type]
            name = r[rank - 1][1]
        except:
            return None
        return name
        
    def getRankAll(self, type):
        if type not in self.attr:
            self.attr[type] = []
            self.save()
        msg = '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n'
        msg += '[0m[47m[30m순  위 존      함    순  위 존      함    순  위 존      함 [0m[37m[40m\r\n'
        msg += '──────────────────────────────\r\n'
        c = 0
        for r in self.attr[type]:
            c += 1
            msg += '[%4d] %-10s    ' % (c, r[1])
            if c % 3 == 0:
                msg += '\r\n'
        msg += '\r\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
        return msg
RANK = Rank()
RANK.load()


