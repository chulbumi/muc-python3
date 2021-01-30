import pickle
from objs.object import Object
from lib.loader import load_script, save_script
import json


class Guild(Object):
    
    attr = {}
    path = 'data/config/guild.dat'
    def __init__(self):
        self.load()
        
    def load(self):
        try:
            f = open(self.path,)
            self.attr = pickle.load(f)
        except IOError:
            print('%s IOError' % self.path)
            return
        except EOFError:
            print('%s EOFError' % self.path)
            return
        except:
            print('Error %s' % self.path)
            return
        f.close()
    
    def save(self):
        try:
            with open(self.path, 'w') as fp:
                json.dump(fp)
        except IOError:
            print('%s IOError' % self.path)
            return
        except EOFError:
            print('%s EOFError' % self.path)
            return
        except:
            print('Error %s' % self.path)
            return

GUILD = Guild()

        